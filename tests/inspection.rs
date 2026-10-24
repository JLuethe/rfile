// SPDX-License-Identifier: GPL-3.0-only
use rfile::{
    CancellationToken, Database, Engine, Error, ExecutionControl, Flags, InspectionBudget, Limits,
    SemanticProfile,
    input::{self, FileView, InputView, InspectionBytes, SliceView},
};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};
fn budget(limits: &Limits) -> InspectionBudget {
    InspectionBudget::new(limits, ExecutionControl::default()).unwrap()
}
fn simple() -> Engine {
    Engine::new(Arc::new(
        Database::from_source("0 string TEST sample\n").unwrap(),
    ))
}
#[test]
fn clones_share_steps_and_work() {
    let l = Limits {
        steps: 3,
        work_bytes: 5,
        ..Limits::default()
    };
    let b = budget(&l);
    let child = b.clone();
    child.steps(2).unwrap();
    b.step().unwrap();
    let e = child.step().unwrap_err().limit_detail().unwrap();
    assert_eq!((e.resource, e.limit, e.observed), ("steps", 3, 4));
    child.bytes(3).unwrap();
    b.bytes(2).unwrap();
    assert_eq!(b.bytes(1).unwrap_err().limit_detail().unwrap().observed, 6);
    assert_eq!(b.metrics().steps, 3);
}
#[test]
fn reservations_release_and_cannot_overflow() {
    let l = Limits {
        memory: 32,
        ..Limits::default()
    };
    let b = budget(&l);
    let lease = b.reserve(32).unwrap();
    assert!(b.reserve(1).unwrap_err().is_limit());
    assert_eq!(b.metrics().tracked_memory_live, 32);
    drop(lease);
    assert_eq!(b.metrics().tracked_memory_live, 0);
    assert!(b.reserve(usize::MAX).unwrap_err().is_limit());
    assert_eq!(b.metrics().tracked_memory_peak, 32);
}
#[test]
fn reservations_can_drop_on_other_threads_with_one_shared_ceiling() {
    let l = Limits {
        memory: 128,
        ..Limits::default()
    };
    let b = budget(&l);
    let ready = Arc::new(Barrier::new(9));
    let release = Arc::new(Barrier::new(9));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let lease = b.reserve(16).unwrap();
            let ready = ready.clone();
            let release = release.clone();
            std::thread::spawn(move || {
                ready.wait();
                release.wait();
                drop(lease);
            })
        })
        .collect();
    ready.wait();
    assert_eq!(b.metrics().tracked_memory_live, 128);
    assert!(b.reserve(1).unwrap_err().is_limit());
    release.wait();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(b.metrics().tracked_memory_live, 0);
}
#[test]
fn failed_buffer_growth_leaves_owned_bytes_and_accounting_intact() {
    let l = Limits {
        memory: 16,
        ..Limits::default()
    };
    let b = budget(&l);
    let mut bytes = InspectionBytes::zeroed(16, &b).unwrap();
    assert!(bytes.extend(&[1]).unwrap_err().is_limit());
    assert_eq!(&*bytes, &[0; 16]);
    drop(bytes);
    assert_eq!(b.metrics().tracked_memory_live, 0);
}
struct NeverRead;
impl Read for NeverRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("cancelled read executed")
    }
}
#[test]
fn cancellation_and_expired_deadline_precede_input_io() {
    let c = CancellationToken::default();
    c.cancel();
    let e = simple().with_control(ExecutionControl {
        cancellation: Some(c),
        deadline: None,
    });
    assert!(matches!(
        e.classify_reader(&mut NeverRead),
        Err(Error::Cancelled)
    ));
    let e = simple().with_control(ExecutionControl {
        deadline: Some(Instant::now() - Duration::from_secs(1)),
        cancellation: None,
    });
    assert!(matches!(e.classify(b"TEST"), Err(Error::DeadlineExceeded)));
}
#[test]
fn cancellation_reaches_budget_clones() {
    let c = CancellationToken::default();
    let b = InspectionBudget::new(
        &Limits::default(),
        ExecutionControl {
            cancellation: Some(c.clone()),
            deadline: None,
        },
    )
    .unwrap();
    let child = b.clone();
    c.cancel();
    assert!(matches!(child.bytes(1), Err(Error::Cancelled)));
}
#[test]
fn slice_ranges_are_checked_and_short_reads_are_not_zero_filled() {
    let b = budget(&Limits::default());
    let mut s = SliceView::new(b"abc");
    assert_eq!(&*input::read_range(&mut s, 1, 10, &b).unwrap(), b"bc");
    assert!(matches!(
        input::read_range(&mut s, u64::MAX, 2, &b),
        Err(Error::Malformed { .. })
    ));
}
struct ShortView;
impl InputView for ShortView {
    fn known_len(&self) -> Option<u64> {
        Some(8)
    }
    fn read_at_unaccounted(&mut self, offset: u64, out: &mut [u8]) -> rfile::Result<usize> {
        if offset == 0 {
            out[..2].copy_from_slice(b"AB");
            Ok(2)
        } else {
            Ok(0)
        }
    }
}
#[test]
fn random_access_short_read_returns_only_observed_bytes() {
    let b = budget(&Limits::default());
    assert_eq!(
        &*input::read_range(&mut ShortView, 0, 8, &b).unwrap(),
        b"AB"
    );
}
struct Interrupted;
impl Read for Interrupted {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::Interrupted.into())
    }
}
#[test]
fn repeated_interruptions_are_bounded_by_io_requests() {
    let l = Limits {
        io_reads: 3,
        ..Limits::default()
    };
    let b = budget(&l);
    let e = input::read_prefix(&mut Interrupted, 1, &b)
        .unwrap_err()
        .limit_detail()
        .unwrap();
    assert_eq!((e.resource, e.limit, e.observed), ("IO requests", 3, 4));
}
#[test]
fn positional_file_reads_do_not_move_a_borrowed_cursor() {
    let p = std::env::temp_dir().join(format!("rfile-position-{}", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&p)
        .unwrap();
    f.write_all(b"01234567").unwrap();
    f.seek(SeekFrom::Start(5)).unwrap();
    let b = budget(&Limits::default());
    let bytes = input::read_range(&mut FileView::new(&f).unwrap(), 1, 3, &b).unwrap();
    assert_eq!(&*bytes, b"123");
    assert_eq!(f.stream_position().unwrap(), 5);
    drop(f);
    std::fs::remove_file(p).unwrap();
}
#[test]
fn malformed_configuration_setter_is_transactional_without_recursion() {
    let mut l = Limits {
        names: 151,
        ..Limits::default()
    };
    let old = l.work_bytes;
    assert!(l.set("work", 0).is_err());
    assert_eq!(l.work_bytes, old);
    l.set("name", 150).unwrap();
    assert_eq!(l.names, 150);
}
#[test]
fn decompression_and_both_classification_passes_share_work() {
    let plain = b"TEST hello world\n";
    let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    writer.write_all(plain).unwrap();
    let compressed = writer.finish().unwrap();
    let mut e = simple();
    e.flags = Flags(Flags::COMPRESS);
    let metrics = e.classify(&compressed).unwrap().metrics;
    let ceiling = metrics.work_bytes - 1;
    assert!(simple().classify(plain).unwrap().metrics.work_bytes < ceiling);
    assert!(simple().classify(&compressed).unwrap().metrics.work_bytes < ceiling);
    e.limits.work_bytes = ceiling;
    assert!(e.classify(&compressed).unwrap_err().is_limit());
}
#[test]
fn capabilities_report_selected_profile_and_real_missing_features() {
    let e = simple().with_profile(SemanticProfile::CorrectedV1);
    let c = e.capabilities();
    assert_eq!(c.profile, SemanticProfile::CorrectedV1);
    assert!(!c.c_abi && !c.kernel_sandbox && !c.regex_backreferences);
    let json: serde_json::Value = serde_json::from_str(&c.json()).unwrap();
    assert_eq!(json["profile"], "corrected-v1");
    assert_eq!(json["sandbox"]["enforced"], false);
}

struct InterruptedView;
impl InputView for InterruptedView {
    fn known_len(&self) -> Option<u64> {
        Some(10)
    }
    fn read_at_unaccounted(&mut self, _: u64, _: &mut [u8]) -> rfile::Result<usize> {
        Err(io::Error::from(io::ErrorKind::Interrupted).into())
    }
}
#[test]
fn range_driver_bounds_interruptions_even_when_adapter_does_no_accounting() {
    let b = budget(&Limits {
        io_reads: 3,
        ..Limits::default()
    });
    let e = input::read_range(&mut InterruptedView, 0, 1, &b).unwrap_err();
    let detail = e.limit_detail().unwrap();
    assert_eq!(
        (detail.resource, detail.limit, detail.observed),
        ("IO requests", 3, 4)
    );
}
