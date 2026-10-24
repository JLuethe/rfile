// SPDX-License-Identifier: GPL-3.0-only
//! Per-inspection resource accounting. Clones share counters, never reset them.
use crate::{Error, LimitViolation, Limits, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Instant;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
#[derive(Clone, Debug, Default)]
pub struct ExecutionControl {
    pub cancellation: Option<CancellationToken>,
    pub deadline: Option<Instant>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InspectionMetrics {
    pub steps: usize,
    pub work_bytes: usize,
    pub io_requests: usize,
    pub io_requested_bytes: usize,
    pub tracked_memory_live: usize,
    pub tracked_memory_peak: usize,
}
#[derive(Debug, Default)]
struct Counters {
    steps: Cell<usize>,
    work: Cell<usize>,
    reads: Cell<usize>,
    read_bytes: Cell<usize>,
}
#[derive(Debug, Default)]
struct MemoryCounters {
    memory: AtomicUsize,
    peak: AtomicUsize,
}
/// Thread-confined accounting for one synchronous inspection. Engines remain
/// Send + Sync; concurrent engine calls create independent inspection counters.
#[derive(Clone, Debug)]
pub struct InspectionBudget {
    pub(crate) limits: Arc<Limits>,
    counters: Rc<Counters>,
    memory: Arc<MemoryCounters>,
    control: ExecutionControl,
}
impl InspectionBudget {
    pub fn new(limits: &Limits, control: ExecutionControl) -> Result<Self> {
        limits.validate()?;
        let budget = Self {
            limits: Arc::new(limits.clone()),
            counters: Rc::default(),
            memory: Arc::default(),
            control,
        };
        budget.checkpoint()?;
        Ok(budget)
    }
    pub fn checkpoint(&self) -> Result<()> {
        if self
            .control
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(Error::Cancelled);
        }
        if self.control.deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(Error::DeadlineExceeded);
        }
        Ok(())
    }
    fn charge(
        counter: &Cell<usize>,
        n: usize,
        limit: usize,
        resource: &'static str,
    ) -> Result<usize> {
        let old = counter.get();
        let next = old.checked_add(n).filter(|&v| v <= limit).ok_or_else(|| {
            Error::ResourceLimit(LimitViolation {
                resource,
                limit,
                observed: old.saturating_add(n),
            })
        })?;
        counter.set(next);
        Ok(next)
    }
    fn charge_memory(&self, n: usize) -> Result<usize> {
        let mut old = self.memory.memory.load(Ordering::Relaxed);
        loop {
            let next = old
                .checked_add(n)
                .filter(|&v| v <= self.limits.memory)
                .ok_or_else(|| {
                    Error::ResourceLimit(LimitViolation {
                        resource: "inspection memory",
                        limit: self.limits.memory,
                        observed: old.saturating_add(n),
                    })
                })?;
            match self.memory.memory.compare_exchange_weak(
                old,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.memory.peak.fetch_max(next, Ordering::Relaxed);
                    return Ok(next);
                }
                Err(current) => old = current,
            }
        }
    }
    pub fn step(&self) -> Result<()> {
        self.steps(1)
    }
    pub fn steps(&self, n: usize) -> Result<()> {
        let current = Self::charge(&self.counters.steps, n, self.limits.steps, "steps")?;
        if current <= n || current / 1024 != (current - n) / 1024 {
            self.checkpoint()?;
        }
        Ok(())
    }
    pub fn bytes(&self, n: usize) -> Result<()> {
        self.checkpoint()?;
        Self::charge(&self.counters.work, n, self.limits.work_bytes, "work bytes")?;
        Ok(())
    }
    pub fn record_read(&self, requested: usize) -> Result<()> {
        self.checkpoint()?;
        self.step()?;
        Self::charge(&self.counters.reads, 1, self.limits.io_reads, "IO requests")?;
        Self::charge(
            &self.counters.read_bytes,
            requested,
            self.limits.io_bytes,
            "IO bytes",
        )?;
        self.bytes(requested)
    }
    pub fn reserve(&self, bytes: usize) -> Result<MemoryReservation> {
        self.checkpoint()?;
        self.charge_memory(bytes)?;
        Ok(MemoryReservation {
            counters: self.memory.clone(),
            bytes,
        })
    }
    /// Conservatively retain allocation charges to classification end while
    /// legacy evaluator vectors migrate to individually tracked buffers.
    pub(crate) fn charge_retained(&self, bytes: usize) -> Result<()> {
        self.checkpoint()?;
        self.charge_memory(bytes)?;
        Ok(())
    }
    pub fn metrics(&self) -> InspectionMetrics {
        InspectionMetrics {
            steps: self.counters.steps.get(),
            work_bytes: self.counters.work.get(),
            io_requests: self.counters.reads.get(),
            io_requested_bytes: self.counters.read_bytes.get(),
            tracked_memory_live: self.memory.memory.load(Ordering::Relaxed),
            tracked_memory_peak: self.memory.peak.load(Ordering::Relaxed),
        }
    }
}
#[derive(Debug)]
pub struct MemoryReservation {
    counters: Arc<MemoryCounters>,
    bytes: usize,
}
impl MemoryReservation {
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
impl Drop for MemoryReservation {
    fn drop(&mut self) {
        self.counters
            .memory
            .fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
