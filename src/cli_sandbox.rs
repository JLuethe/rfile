// SPDX-License-Identifier: GPL-3.0-only
//! CLI-only confinement lifecycle. Engine APIs never enter this module.
use super::*;
pub(super) const COMPILED: bool = cfg!(all(
    feature = "sandbox",
    target_os = "linux",
    target_env = "gnu",
    target_arch = "x86_64"
));
fn status_off(plan: &CliPlan, reason: &str) {
    if plan.sandbox_status {
        eprintln!(
            "{}",
            serde_json::json!({"sandbox": {"mode": plan.sandbox.name(), "compiled": COMPILED, "seccomp_enforced": false, "landlock_enforced": false, "input_content_confined": false, "reason": reason}})
        );
    }
}
pub(super) fn execute(plan: CliPlan) -> Result<i32> {
    if plan.sandbox == SandboxMode::Off {
        status_off(&plan, "explicitly disabled");
        return execute_events(plan.events);
    }
    #[cfg(all(
        feature = "sandbox",
        target_os = "linux",
        target_env = "gnu",
        target_arch = "x86_64"
    ))]
    {
        linux::execute(plan)
    }
    #[cfg(not(all(
        feature = "sandbox",
        target_os = "linux",
        target_env = "gnu",
        target_arch = "x86_64"
    )))]
    {
        if plan.sandbox == SandboxMode::Strict {
            return Err(Error::Unsupported(
                "strict sandbox is not compiled for this target".into(),
            ));
        }
        status_off(&plan, "not compiled for this target");
        execute_events(plan.events)
    }
}
#[cfg(all(
    feature = "sandbox",
    target_os = "linux",
    target_env = "gnu",
    target_arch = "x86_64"
))]
mod linux {
    use super::*;
    use crate::{InspectionBudget, filesystem::PreparedPath};
    use rfile_platform::sandbox::{Policy, SandboxPlan, Status};

    fn report(plan: &CliPlan, status: &Status, preload: bool) {
        for reason in &status.degraded {
            eprintln!("rfile: sandbox degraded: {reason}");
        }
        if preload {
            eprintln!(
                "rfile: sandbox: LD_PRELOAD is present; confinement cannot undo code executed before main"
            );
        }
        if plan.sandbox_status {
            eprintln!(
                "{}",
                serde_json::json!({"sandbox": {"mode": plan.sandbox.name(), "compiled": true,
                "seccomp_available": status.seccomp_available, "seccomp_enforced": status.seccomp_enforced,
                "landlock_abi": status.landlock_abi, "landlock_enforced": status.landlock_enforced,
                "handled_fs_rights": status.handled_fs_rights, "policy": status.policy,
                "input_content_confined": status.input_content_confined, "degraded": status.degraded,
                "scope": "current thread; verified single-threaded CLI startup", "ld_preload": preload}})
            );
        }
    }
    pub(super) fn execute(plan: CliPlan) -> Result<i32> {
        let work = plan.events.iter().any(|e| {
            matches!(
                e,
                Event::List { .. }
                    | Event::Operands { .. }
                    | Event::Database { .. }
                    | Event::Capabilities { .. }
            )
        });
        if !work && !plan.sandbox_status {
            // Help/version/argument errors do not parse external inputs.
            return execute_events(plan.events);
        }
        let strict = plan.sandbox == SandboxMode::Strict;
        let preload = std::env::var_os("LD_PRELOAD").is_some_and(|s| !s.is_empty());
        if strict && preload {
            return Err(Error::Unsupported("strict sandbox with LD_PRELOAD".into()));
        }
        rfile_platform::sandbox::ensure_single_thread()?;
        let inherited = rfile_platform::sandbox::inherited_descriptors()?;
        let compile = plan
            .events
            .iter()
            .any(|e| matches!(e, Event::Database { action: 'C', .. }));
        let classify = plan
            .events
            .iter()
            .any(|e| matches!(e, Event::List { .. } | Event::Operands { .. }));
        if compile && classify {
            return Err(Error::Unsupported(
                "sandbox mixed classification and compilation actions; use separate invocations"
                    .into(),
            ));
        }
        let preserve_times = plan.events.iter().any(|e| match e {
            Event::List { cookie, .. } | Event::Operands { cookie, .. } => {
                cookie.flags.has(Flags::PRESERVE_ATIME)
            }
            _ => false,
        });
        let policy = if compile {
            Policy::Compile {
                directories: vec![std::env::current_dir()?],
            }
        } else {
            Policy::ReadOnly { preserve_times }
        };
        let status = SandboxPlan::prepare(&policy, strict, &inherited)?.install()?;
        if strict && classify {
            // All context/database parsing and path resolution occur under the
            // baseline. No input contents are detected before final tightening.
            let prepared = prepare(&plan.events)?;
            let mut reads = Vec::new();
            let mut times = Vec::new();
            for e in &prepared {
                if let PreparedEvent::Inputs { engine, inputs, .. } = e {
                    for (_, _, input) in inputs {
                        let fd = match input {
                            Input::Stdin => Some(0),
                            Input::Path(p, _) => p.descriptor(),
                        };
                        if let Some(fd) = fd {
                            reads.push(fd);
                            if engine.flags.has(Flags::PRESERVE_ATIME) && fd != 0 {
                                times.push(fd);
                            }
                        }
                    }
                }
            }
            reads.sort_unstable();
            reads.dedup();
            times.sort_unstable();
            times.dedup();
            let status = SandboxPlan::prepare(
                &Policy::Preopened {
                    read_fds: reads,
                    timestamp_fds: times,
                },
                true,
                &inherited,
            )?
            .install()?;
            report(&plan, &status, preload);
            execute_prepared(prepared)
        } else {
            report(&plan, &status, preload);
            execute_events(plan.events)
        }
    }
    enum Input {
        Stdin,
        Path(Box<PreparedPath>, InspectionBudget),
    }
    enum PreparedEvent {
        Inputs {
            engine: Box<Engine>,
            presentation: Presentation,
            inputs: Vec<(OsString, usize, Input)>,
        },
        Text(String),
        End,
        Help,
        Error(String),
    }
    fn prepare(events: &[Event]) -> Result<Vec<PreparedEvent>> {
        let mut loaded = None;
        let mut result = Vec::new();
        let mut count = 0usize;
        for event in events {
            match event {
                Event::List {
                    cookie,
                    limits,
                    path,
                    presentation,
                } => {
                    if path == OsStr::new("-") || presentation.no_buffer {
                        return Err(Error::Unsupported("strict sandbox requires bounded regular file lists without --no-buffer; stdin filename streams are unsupported".into()));
                    }
                    let engine = event_engine(&mut loaded, cookie, limits)?.clone();
                    let mut options = std::fs::OpenOptions::new();
                    use std::os::unix::fs::OpenOptionsExt;
                    options.read(true).custom_flags(0x800); // Linux O_NONBLOCK: reject FIFO lists without waiting.
                    let file = options.open(path)?;
                    if !file.metadata()?.is_file() {
                        return Err(Error::Unsupported(
                            "strict sandbox requires a regular filename list".into(),
                        ));
                    }
                    let mut reader = io::BufReader::new(file);
                    let mut names = Vec::new();
                    loop {
                        let mut line = Vec::new();
                        if reader.by_ref().take(65537).read_until(b'\n', &mut line)? == 0 {
                            break;
                        }
                        if line.len() > 65536 {
                            return Err(Error::Limit("file-list line"));
                        }
                        if line.last() == Some(&b'\n') {
                            line.pop();
                        }
                        names.push(os_bytes(line));
                        if names.len() + count > 128 {
                            return Err(Error::Limit("strict sandbox inputs (128)"));
                        }
                    }
                    result.push(inputs(engine, names, presentation.clone(), &mut count)?);
                }
                Event::Operands {
                    cookie,
                    limits,
                    names,
                    presentation,
                } => {
                    let engine = event_engine(&mut loaded, cookie, limits)?.clone();
                    result.push(inputs(
                        engine,
                        names.clone(),
                        presentation.clone(),
                        &mut count,
                    )?);
                }
                Event::Help => {
                    result.push(PreparedEvent::Help);
                    break;
                }
                Event::Version { path } => {
                    result.push(PreparedEvent::Text(format!("rfile-{}\nmagic v{} from {}\nupstream {}\nfeatures: safe-rust regex miniz_oxide; kernel sandbox compiled={}\n", env!("CARGO_PKG_VERSION"), crate::FORMAT_VERSION, path.as_ref().map_or("embedded pinned database".into(), |p| p.to_string_lossy()), crate::UPSTREAM_COMMIT, COMPILED)));
                    result.push(PreparedEvent::End);
                    break;
                }
                Event::Capabilities { profile } => {
                    let mut c = Capabilities::current(*profile);
                    c.time_zone = crate::TimeContext::from_environment()?.source().to_owned();
                    c.display_locale = crate::DisplayContext::from_environment()?.name().to_owned();
                    result.push(PreparedEvent::Text(format!("{}\n", c.json())));
                    result.push(PreparedEvent::End);
                    break;
                }
                // Errors cannot be cloned; preserve their text and ordered position.
                Event::Error(e) => {
                    result.push(PreparedEvent::Error(e.to_string()));
                    break;
                }
                Event::Database { .. } | Event::Usage => {
                    return Err(Error::Unsupported(
                        "strict sandbox mixed action plan".into(),
                    ));
                }
            }
        }
        Ok(result)
    }
    fn inputs(
        engine: Engine,
        names: Vec<OsString>,
        presentation: Presentation,
        count: &mut usize,
    ) -> Result<PreparedEvent> {
        *count += names.len();
        if *count > 128 {
            return Err(Error::Limit("strict sandbox inputs (128)"));
        }
        let width = names
            .iter()
            .try_fold(0usize, |n, s| display_width(s, &engine).map(|w| n.max(w)))?;
        let mut inputs = Vec::new();
        for name in names {
            let input = if name == OsStr::new("-") {
                Input::Stdin
            } else {
                let budget = engine.budget()?;
                let path = crate::filesystem::prepare_path(&engine, Path::new(&name), &budget);
                Input::Path(Box::new(path), budget)
            };
            inputs.push((name, width, input));
        }
        Ok(PreparedEvent::Inputs {
            engine: Box::new(engine),
            presentation,
            inputs,
        })
    }
    fn execute_prepared(events: Vec<PreparedEvent>) -> Result<i32> {
        let mut failed = false;
        for event in events {
            match event {
                PreparedEvent::Inputs {
                    engine,
                    presentation,
                    inputs,
                } => {
                    for (name, width, input) in inputs {
                        let classified = match input {
                            Input::Stdin => engine.classify_reader(&mut io::stdin().lock()),
                            Input::Path(path, budget) => {
                                engine.check_output(path.classify(&engine, &budget), &budget)
                            }
                        };
                        failed |= emit_operand(&engine, &name, width, &presentation, classified)?;
                    }
                }
                PreparedEvent::Text(text) => print!("{text}"),
                PreparedEvent::End => return Ok(0),
                PreparedEvent::Help => {
                    help();
                    return Ok(0);
                }
                PreparedEvent::Error(e) => {
                    eprintln!("rfile: {e}");
                    return Ok(1);
                }
            }
        }
        io::stdout().flush()?;
        Ok(i32::from(failed))
    }
}
