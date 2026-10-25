// SPDX-License-Identifier: GPL-3.0-only
#![cfg(all(
    feature = "sandbox",
    target_os = "linux",
    target_env = "gnu",
    target_arch = "x86_64"
))]
use rfile_platform::sandbox::{Policy, SandboxPlan, inherited_descriptors};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
    process::Command,
};

#[test]
fn kernel_policy_children() {
    match SandboxPlan::prepare(
        &Policy::ReadOnly {
            preserve_times: false,
        },
        true,
        &[],
    ) {
        Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
            eprintln!("SKIP kernel policy certification: {e}");
            return;
        }
        Err(e) => panic!("kernel setup failed: {e}"),
        Ok(_) => {}
    }
    for case in [
        "readonly",
        "preopened",
        "compile",
        "inherited",
        "denied-setup",
        "unavailable",
        "identity",
        "range-gaps",
    ] {
        let root =
            std::env::temp_dir().join(format!("rfile-sandbox-{}-{case}", std::process::id()));
        fs::create_dir_all(root.join("output")).unwrap();
        fs::write(root.join("input"), b"hello kernel policy\n").unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "policy_child", "--nocapture"])
            .env("RFILE_SANDBOX_TEST_CASE", case)
            .env("RFILE_SANDBOX_TEST_ROOT", &root)
            .output()
            .unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(
            child.status.success(),
            "{case}: {} {}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(
            child.stdout.windows(11).any(|s| s == b"POLICY PASS"),
            "child did not run {case}"
        );
    }
}
fn denied<T>(result: std::io::Result<T>) {
    assert!(matches!(
        result.err().and_then(|e| e.raw_os_error()),
        Some(libc::EPERM | libc::EACCES)
    ));
}
#[test]
fn policy_child() {
    let Ok(case) = std::env::var("RFILE_SANDBOX_TEST_CASE") else {
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("RFILE_SANDBOX_TEST_ROOT").unwrap());
    let input = root.join("input");
    if case == "denied-setup" || case == "unavailable" {
        use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch};
        let errno = if case == "unavailable" {
            libc::ENOSYS
        } else {
            libc::EPERM
        };
        let program: BpfProgram = SeccompFilter::new(
            [(libc::SYS_landlock_create_ruleset, vec![])].into(),
            SeccompAction::Allow,
            SeccompAction::Errno(errno as u32),
            TargetArch::x86_64,
        )
        .unwrap()
        .try_into()
        .unwrap();
        seccompiler::apply_filter(&program).unwrap();
        let policy = Policy::ReadOnly {
            preserve_times: false,
        };
        let strict = SandboxPlan::prepare(&policy, true, &[])
            .err()
            .expect("strict setup must fail");
        if case == "denied-setup" {
            assert_eq!(strict.raw_os_error(), Some(libc::EPERM));
            assert!(SandboxPlan::prepare(&policy, false, &[]).is_err());
        } else {
            assert_eq!(strict.kind(), std::io::ErrorKind::Unsupported);
            let status = SandboxPlan::prepare(&policy, false, &[])
                .unwrap()
                .install()
                .unwrap();
            assert!(
                status.seccomp_enforced
                    && !status.landlock_enforced
                    && !status.input_content_confined
            );
            assert!(!status.degraded.is_empty());
        }
        println!("POLICY PASS");
        return;
    }
    if case == "range-gaps" {
        let mut first = File::open(&input).unwrap();
        let mut forbidden = File::open(&input).unwrap();
        let mut last = File::open(&input).unwrap();
        assert!(
            first.as_raw_fd() < forbidden.as_raw_fd() && forbidden.as_raw_fd() < last.as_raw_fd()
        );
        let policy = Policy::Preopened {
            read_fds: vec![first.as_raw_fd(), last.as_raw_fd()],
            timestamp_fds: vec![],
        };
        SandboxPlan::prepare(&policy, true, &[])
            .unwrap()
            .install()
            .unwrap();
        let mut byte = [0];
        assert_eq!(first.read(&mut byte).unwrap(), 1);
        assert_eq!(last.read(&mut byte).unwrap(), 1);
        denied(forbidden.read(&mut byte));
        println!("POLICY PASS");
        return;
    }
    if case == "identity" {
        let mut file = File::open(&input).unwrap();
        fs::rename(&input, root.join("old")).unwrap();
        fs::write(&input, b"replacement").unwrap();
        fs::remove_file(root.join("old")).unwrap();
        let status = SandboxPlan::prepare(
            &Policy::Preopened {
                read_fds: vec![file.as_raw_fd()],
                timestamp_fds: vec![],
            },
            true,
            &[],
        )
        .unwrap()
        .install()
        .unwrap();
        assert!(status.input_content_confined);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"hello kernel policy\n");
        denied(File::open(&input));
        denied(File::open(format!("/proc/self/fd/{}", file.as_raw_fd())));
        println!("POLICY PASS");
        return;
    }

    let mut inherited = if case == "inherited" {
        Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&input)
                .unwrap(),
        )
    } else {
        None
    };
    let blocked = inherited_descriptors().unwrap();
    let policy = if case == "compile" {
        Policy::Compile {
            directories: vec![root.join("output")],
        }
    } else {
        Policy::ReadOnly {
            preserve_times: true,
        }
    };
    let status = SandboxPlan::prepare(&policy, true, &blocked)
        .unwrap()
        .install()
        .unwrap();
    assert!(status.landlock_enforced && status.seccomp_enforced);
    assert!(status.landlock_abi.unwrap() >= 5);
    if case == "inherited" {
        let f = inherited.as_mut().unwrap();
        denied(f.write_all(b"bad"));
        denied(f.read(&mut [0u8; 1]));
        // SAFETY: No returned memory is dereferenced. A successful unexpected
        // mapping is released immediately; the file remains owned for the call.
        let mapped = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                f.as_raw_fd(),
                0,
            )
        };
        if mapped != libc::MAP_FAILED {
            // SAFETY: This is the exact successful mapping and length above.
            unsafe { libc::munmap(mapped, 4096) };
            panic!("inherited mapping allowed");
        }
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
    } else if case == "preopened" {
        let mut f = File::open(&input).unwrap();
        let status = SandboxPlan::prepare(
            &Policy::Preopened {
                read_fds: vec![f.as_raw_fd()],
                timestamp_fds: vec![f.as_raw_fd()],
            },
            true,
            &blocked,
        )
        .unwrap()
        .install()
        .unwrap();
        assert!(status.input_content_confined);
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        assert_eq!(b, b"hello kernel policy\n");
        f.set_times(fs::FileTimes::new().set_accessed(std::time::SystemTime::now()))
            .unwrap();
        denied(File::open(&input));
        denied(File::open("/etc/passwd"));
    } else if case == "compile" {
        fs::write(root.join("output/result"), b"compiled").unwrap();
        fs::rename(root.join("output/result"), root.join("output/final")).unwrap();
        denied(fs::write(root.join("outside"), b"bad"));
        denied(fs::rename(root.join("output/final"), root.join("outside")));
    } else {
        assert_eq!(fs::read(&input).unwrap(), b"hello kernel policy\n");
        denied(fs::write(root.join("new"), b"bad"));
        denied(fs::remove_file(&input));
        // O_RDONLY|O_CREAT must not slip through a read-only access-mode check.
        let name =
            std::ffi::CString::new(root.join("readonly-create").as_os_str().as_encoded_bytes())
                .unwrap();
        // SAFETY: CString is NUL terminated and remains alive during open.
        let fd = unsafe { libc::open(name.as_ptr(), libc::O_RDONLY | libc::O_CREAT, 0o600) };
        assert_eq!(fd, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
        // Reading directories remains allowed despite O_TMPFILE sharing its bit.
        assert!(fs::read_dir(&root).unwrap().next().is_some());
    }
    // Unknown/runtime-unneeded operations and descriptor duplication are denied.
    // SAFETY: These integer-only calls retain no pointers; -1 is required.
    assert_eq!(unsafe { libc::dup(1) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EPERM)
    );
    // SAFETY: No pointer argument or memory ownership transfer.
    assert_eq!(unsafe { libc::syscall(libc::SYS_getpriority, 0, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EPERM)
    );
    denied(std::net::TcpListener::bind("127.0.0.1:0"));
    denied(Command::new("/bin/true").status());
    println!("POLICY PASS");
}
