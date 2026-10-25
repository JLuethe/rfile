// SPDX-License-Identifier: GPL-3.0-only
//! Explicit current-thread kernel restrictions. Never installed by engine calls.
//! Prepared filters are immutable; installation is irreversible and consumes the
//! plan. A returned status does not attest to another thread's confinement.
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr, RulesetStatus,
};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};
use std::{collections::BTreeMap, io, marker::PhantomData, path::PathBuf, rc::Rc};

#[derive(Debug)]
pub enum Policy {
    ReadOnly {
        preserve_times: bool,
    },
    Compile {
        directories: Vec<PathBuf>,
    },
    Preopened {
        read_fds: Vec<i32>,
        timestamp_fds: Vec<i32>,
    },
}
#[derive(Debug)]
pub struct Status {
    pub seccomp_available: bool,
    pub seccomp_enforced: bool,
    pub landlock_abi: Option<i32>,
    pub landlock_enforced: bool,
    pub handled_fs_rights: u64,
    pub input_content_confined: bool,
    pub policy: &'static str,
    pub degraded: Vec<String>,
    _thread: PhantomData<Rc<()>>,
}
pub struct SandboxPlan {
    landlock: Option<RulesetCreated>,
    filter: Option<BpfProgram>,
    status: Status,
}
fn invalid(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

/// Inventory rather than close foreign descriptors: closing an FD potentially
/// owned by Rust would violate IO safety. Filters deny authority on these FDs.
/// CLI startup must be single-threaded while taking this snapshot.
pub fn inherited_descriptors() -> io::Result<Vec<i32>> {
    let names = std::fs::read_dir("/proc/self/fd")?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().parse::<i32>().ok()))
        .collect::<io::Result<Vec<_>>>()?;
    let mut fds = Vec::new();
    for fd in names.into_iter().flatten().filter(|&n| n > 2) {
        // SAFETY: F_GETFD takes an integer descriptor and no pointer; invalid or
        // concurrently closed descriptors return EBADF without Rust borrowing.
        if unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0 {
            fds.push(fd);
        }
    }
    fds.sort_unstable();
    fds.dedup();
    if fds.len() > 32 {
        return Err(invalid("more than 32 inherited descriptors"));
    }
    Ok(fds)
}
pub fn ensure_single_thread() -> io::Result<()> {
    if std::fs::read_dir("/proc/self/task")?
        .collect::<io::Result<Vec<_>>>()?
        .len()
        != 1
    {
        return Err(invalid("CLI confinement requires single-threaded startup"));
    }
    Ok(())
}
fn query() -> io::Result<(bool, Option<i32>)> {
    // SAFETY: PR_GET_SECCOMP has no pointer arguments and does not mutate policy.
    let s = unsafe { libc::prctl(libc::PR_GET_SECCOMP, 0, 0, 0, 0) };
    let seccomp = if s >= 0 {
        true
    } else {
        let e = io::Error::last_os_error();
        if matches!(e.raw_os_error(), Some(libc::EINVAL | libc::ENOSYS)) {
            false
        } else {
            return Err(e);
        }
    };
    // SAFETY: VERSION query explicitly requires NULL and zero size; no buffer
    // is dereferenced. Preserve unexpected errno instead of treating it as absent.
    let l = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<u8>(),
            0usize,
            1u32,
        )
    };
    let landlock = if l >= 0 {
        Some(l as i32)
    } else {
        let e = io::Error::last_os_error();
        if matches!(e.raw_os_error(), Some(libc::ENOSYS | libc::EOPNOTSUPP)) {
            None
        } else {
            return Err(e);
        }
    };
    Ok((seccomp, landlock))
}
impl SandboxPlan {
    pub fn prepare(policy: &Policy, required: bool, inherited: &[i32]) -> io::Result<Self> {
        if inherited.len() > 32 || inherited.iter().any(|&fd| fd < 3) {
            return Err(invalid("invalid inherited descriptor inventory"));
        }
        if let Policy::Preopened {
            read_fds,
            timestamp_fds,
        } = policy
        {
            if read_fds.len() > 128
                || timestamp_fds.len() > 128
                || read_fds.iter().any(|&fd| fd < 0)
                || timestamp_fds.iter().any(|fd| !read_fds.contains(fd))
            {
                return Err(invalid("invalid preopened descriptor set"));
            }
        }
        let (seccomp, abi) = query()?;
        // V5 includes truncation, rename/refer and device ioctl rights. Later
        // kernel rights are not silently claimed by this fixed policy.
        let supported = abi.is_some_and(|n| n >= 5);
        if (required || matches!(policy, Policy::Compile { .. })) && (!seccomp || !supported) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "required confinement needs seccomp and Landlock ABI >= 5",
            ));
        }
        let rights = AccessFs::from_all(ABI::V5);
        let mut status = Status {
            seccomp_available: seccomp,
            seccomp_enforced: false,
            landlock_abi: abi,
            landlock_enforced: false,
            handled_fs_rights: if supported { rights.bits() } else { 0 },
            input_content_confined: false,
            policy: match policy {
                Policy::ReadOnly { .. } => "read-only",
                Policy::Compile { .. } => "compile",
                Policy::Preopened { .. } => "preopened",
            },
            degraded: Vec::new(),
            _thread: PhantomData,
        };
        if !seccomp {
            status.degraded.push("seccomp unavailable".into());
        }
        if !supported {
            status
                .degraded
                .push("Landlock ABI 5 filesystem policy unavailable".into());
        }
        let rules = if supported {
            let mut rules = Ruleset::default()
                .set_compatibility(CompatLevel::HardRequirement)
                .handle_access(rights)
                .map_err(invalid)?
                .create()
                .map_err(invalid)?;
            if !matches!(policy, Policy::Preopened { .. }) {
                rules = rules
                    .add_rule(PathBeneath::new(
                        PathFd::new("/").map_err(invalid)?,
                        AccessFs::ReadFile | AccessFs::ReadDir,
                    ))
                    .map_err(invalid)?;
            }
            if let Policy::Compile { directories } = policy {
                if directories.len() > 128 {
                    return Err(invalid("too many compile output directories"));
                }
                let write = rights & !(AccessFs::Execute | AccessFs::IoctlDev);
                for path in directories {
                    rules = rules
                        .add_rule(PathBeneath::new(PathFd::new(path).map_err(invalid)?, write))
                        .map_err(invalid)?;
                }
            }
            Some(rules)
        } else {
            None
        };
        let filter = seccomp
            .then(|| build_filter(policy, inherited))
            .transpose()?;
        Ok(Self {
            landlock: rules,
            filter,
            status,
        })
    }
    pub fn install(mut self) -> io::Result<Status> {
        if let Some(rules) = self.landlock {
            let applied = rules.restrict_self().map_err(invalid)?;
            if applied.ruleset != RulesetStatus::FullyEnforced || !applied.no_new_privs {
                return Err(invalid("Landlock policy was not fully enforced"));
            }
            self.status.landlock_enforced = true;
        }
        if let Some(filter) = self.filter {
            seccompiler::apply_filter(&filter).map_err(invalid)?;
            self.status.seccomp_enforced = true;
        }
        self.status.input_content_confined = self.status.policy == "preopened"
            && self.status.landlock_enforced
            && self.status.seccomp_enforced;
        Ok(self.status)
    }
}
fn condition(arg: u8, op: SeccompCmpOp, value: u64) -> io::Result<SeccompCondition> {
    SeccompCondition::new(arg, SeccompCmpArgLen::Dword, op, value).map_err(invalid)
}
fn null_pointer(arg: u8) -> io::Result<SeccompCondition> {
    SeccompCondition::new(arg, SeccompCmpArgLen::Qword, SeccompCmpOp::Eq, 0).map_err(invalid)
}
fn rule(c: Vec<SeccompCondition>) -> io::Result<SeccompRule> {
    SeccompRule::new(c).map_err(invalid)
}
fn exclusions(arg: u8, fds: &[i32]) -> io::Result<Vec<SeccompCondition>> {
    fds.iter()
        .map(|&fd| condition(arg, SeccompCmpOp::Ne, fd as u64))
        .collect()
}
/// Merge only consecutive members of the authorized set. No descriptor absent
/// from that set is admitted. Strict mode forbids opening/duplicating new FDs,
/// so closing an authorized descriptor cannot transfer its authority by reuse.
fn fd_conditions(arg: u8, fds: &[i32]) -> io::Result<Vec<Vec<SeccompCondition>>> {
    let mut fds = fds.to_vec();
    fds.sort_unstable();
    fds.dedup();
    let mut result = Vec::new();
    let mut at = 0;
    while at < fds.len() {
        let start = fds[at];
        let mut end = start;
        at += 1;
        while at < fds.len() && end.checked_add(1) == Some(fds[at]) {
            end = fds[at];
            at += 1;
        }
        result.push(if start == end {
            vec![condition(arg, SeccompCmpOp::Eq, start as u64)?]
        } else {
            vec![
                condition(arg, SeccompCmpOp::Ge, start as u64)?,
                condition(arg, SeccompCmpOp::Le, end as u64)?,
            ]
        });
    }
    Ok(result)
}
fn fd_rules(arg: u8, allow: Option<&[i32]>, blocked: &[i32]) -> io::Result<Vec<SeccompRule>> {
    if let Some(fds) = allow {
        fd_conditions(arg, fds)?.into_iter().map(rule).collect()
    } else if blocked.is_empty() {
        Ok(Vec::new())
    } else {
        Ok(vec![rule(exclusions(arg, blocked)?)?])
    }
}
fn build_filter(policy: &Policy, inherited: &[i32]) -> io::Result<BpfProgram> {
    use libc::*;
    let mut calls: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    let strict = matches!(policy, Policy::Preopened { .. });
    let compile = matches!(policy, Policy::Compile { .. });
    for n in [
        SYS_brk,
        SYS_munmap,
        SYS_madvise,
        SYS_mremap,
        SYS_futex,
        SYS_clock_gettime,
        SYS_getpid,
        SYS_gettid,
        SYS_rt_sigaction,
        SYS_rt_sigprocmask,
        SYS_rt_sigreturn,
        SYS_sigaltstack,
        SYS_exit,
        SYS_exit_group,
        SYS_close,
        SYS_fstat,
        SYS_newfstatat,
        SYS_stat,
        SYS_lstat,
        SYS_statx,
        SYS_readlink,
        SYS_readlinkat,
        SYS_access,
        SYS_faccessat,
        SYS_faccessat2,
        SYS_getrandom,
        SYS_sched_yield,
        SYS_restart_syscall,
    ] {
        calls.insert(n, vec![]);
    }
    let allowed_reads = if let Policy::Preopened { read_fds, .. } = policy {
        if read_fds.len() > 128 || read_fds.iter().any(|&fd| fd < 0) {
            return Err(invalid("invalid preopened descriptor set"));
        }
        Some(read_fds.as_slice())
    } else {
        None
    };
    for n in [
        SYS_read,
        SYS_readv,
        SYS_pread64,
        SYS_preadv,
        SYS_preadv2,
        SYS_lseek,
        SYS_getdents64,
    ] {
        let rules = fd_rules(0, allowed_reads, inherited)?;
        if allowed_reads.is_none_or(|fds| !fds.is_empty()) {
            calls.insert(n, rules);
        }
    }
    for n in [SYS_write, SYS_writev, SYS_pwrite64, SYS_pwritev] {
        calls.insert(
            n,
            fd_rules(0, if compile { None } else { Some(&[1, 2]) }, inherited)?,
        );
    }
    let mut mmap_rules = Vec::new();
    mmap_rules.push(rule(vec![
        condition(2, SeccompCmpOp::MaskedEq(PROT_EXEC as u64), 0)?,
        condition(
            3,
            SeccompCmpOp::MaskedEq(MAP_ANONYMOUS as u64),
            MAP_ANONYMOUS as u64,
        )?,
    ])?);
    if let Some(fds) = allowed_reads {
        for mut c in fd_conditions(4, fds)? {
            c.push(condition(2, SeccompCmpOp::MaskedEq(PROT_EXEC as u64), 0)?);
            c.push(condition(3, SeccompCmpOp::MaskedEq(MAP_SHARED as u64), 0)?);
            mmap_rules.push(rule(c)?);
        }
    } else {
        let mut c = exclusions(4, inherited)?;
        c.push(condition(2, SeccompCmpOp::MaskedEq(PROT_EXEC as u64), 0)?);
        if !compile {
            c.push(condition(3, SeccompCmpOp::MaskedEq(MAP_SHARED as u64), 0)?);
        }
        mmap_rules.push(rule(c)?);
    }
    calls.insert(SYS_mmap, mmap_rules);
    calls.insert(
        SYS_mprotect,
        vec![rule(vec![condition(
            2,
            SeccompCmpOp::MaskedEq(PROT_EXEC as u64),
            0,
        )?])?],
    );
    let mut fcntl = Vec::new();
    for cmd in [F_GETFL, F_GETFD, F_SETFD] {
        let mut c = exclusions(0, inherited)?;
        c.push(condition(1, SeccompCmpOp::Eq, cmd as u64)?);
        fcntl.push(rule(c)?);
    }
    calls.insert(SYS_fcntl, fcntl);
    let mut ioctls = Vec::new();
    for fd in 0..=2 {
        for request in [TCGETS, TIOCGWINSZ] {
            ioctls.push(rule(vec![
                condition(0, SeccompCmpOp::Eq, fd)?,
                condition(1, SeccompCmpOp::Eq, request)?,
            ])?);
        }
    }
    calls.insert(SYS_ioctl, ioctls);
    if !strict {
        let disallowed =
            (O_ACCMODE | O_CREAT | O_TRUNC | O_APPEND | (O_TMPFILE & !O_DIRECTORY)) as u64;
        for (n, arg) in [(SYS_open, 1), (SYS_openat, 2)] {
            let mut c = if n == SYS_openat {
                exclusions(0, inherited)?
            } else {
                Vec::new()
            };
            if !compile {
                c.push(condition(arg, SeccompCmpOp::MaskedEq(disallowed), 0)?);
            }
            calls.insert(n, if c.is_empty() { vec![] } else { vec![rule(c)?] });
        }
        for n in [
            SYS_landlock_create_ruleset,
            SYS_landlock_add_rule,
            SYS_landlock_restrict_self,
        ] {
            calls.insert(n, vec![]);
        }
        calls.insert(
            SYS_prctl,
            vec![
                rule(vec![
                    condition(0, SeccompCmpOp::Eq, PR_SET_NO_NEW_PRIVS as u64)?,
                    condition(1, SeccompCmpOp::Eq, 1)?,
                ])?,
                rule(vec![condition(0, SeccompCmpOp::Eq, PR_GET_SECCOMP as u64)?])?,
            ],
        );
        calls.insert(
            SYS_seccomp,
            vec![rule(vec![
                condition(0, SeccompCmpOp::Eq, SECCOMP_SET_MODE_FILTER as u64)?,
                null_pointer(1)?,
            ])?],
        );
    }
    let time_fds = match policy {
        Policy::Preopened { timestamp_fds, .. } => Some(timestamp_fds.as_slice()),
        Policy::ReadOnly {
            preserve_times: true,
        } => None,
        _ => Some([].as_slice()),
    };
    if time_fds.is_none_or(|fds| !fds.is_empty()) {
        let mut rules = Vec::new();
        if let Some(fds) = time_fds {
            for mut c in fd_conditions(0, fds)? {
                c.push(null_pointer(1)?);
                c.push(condition(3, SeccompCmpOp::Eq, 0)?);
                rules.push(rule(c)?);
            }
        } else {
            let mut c = exclusions(0, inherited)?;
            c.push(null_pointer(1)?);
            c.push(condition(3, SeccompCmpOp::Eq, 0)?);
            rules.push(rule(c)?);
        }
        calls.insert(SYS_utimensat, rules);
    }
    if compile {
        for n in [
            SYS_mkdirat,
            SYS_rename,
            SYS_renameat,
            SYS_renameat2,
            SYS_unlink,
            SYS_unlinkat,
            SYS_fsync,
            SYS_fdatasync,
        ] {
            calls.insert(n, vec![]);
        }
    }
    let program: BpfProgram = SeccompFilter::new(
        calls,
        SeccompAction::Errno(EPERM as u32),
        SeccompAction::Allow,
        TargetArch::x86_64,
    )
    .map_err(invalid)?
    .try_into()
    .map_err(invalid)?;
    if program.len() > 4096 {
        return Err(invalid("seccomp instruction limit"));
    }
    Ok(program)
}

/// Publish compiled data using stable directory descriptors and private staging.
/// The caller chooses the output directory; sandbox compile policy grants only
/// that directory tree. No caller-owned descriptor is consumed or closed.
pub fn write_database(output: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    use rustix::fs::{AtFlags, Mode, OFlags, mkdirat, openat, renameat, unlinkat};
    use std::{
        fs::File,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = output
        .file_name()
        .ok_or_else(|| invalid("compile output requires a basename"))?;
    let parent = File::open(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new(".")),
    )?;
    for _ in 0..32 {
        let staging = format!(
            ".rfile-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        match mkdirat(&parent, &staging, Mode::from_raw_mode(0o700)) {
            Ok(()) => {}
            Err(e) if e == rustix::io::Errno::EXIST => continue,
            Err(e) => return Err(e.into()),
        }
        let directory = match openat(
            &parent,
            &staging,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => File::from(fd),
            Err(e) => {
                let _ = unlinkat(&parent, &staging, AtFlags::REMOVEDIR);
                return Err(e.into());
            }
        };
        let result = (|| -> io::Result<()> {
            let mut file = File::from(openat(
                &directory,
                "database",
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o666),
            )?);
            file.write_all(bytes)?;
            file.sync_all()?;
            renameat(&directory, "database", &parent, name)?;
            parent.sync_all()
        })();
        if result.is_err() {
            let _ = unlinkat(&directory, "database", AtFlags::empty());
        }
        let cleanup = unlinkat(&parent, &staging, AtFlags::REMOVEDIR);
        result?;
        cleanup?;
        parent.sync_all()?;
        return Ok(());
    }
    Err(invalid("compile staging attempt limit"))
}
