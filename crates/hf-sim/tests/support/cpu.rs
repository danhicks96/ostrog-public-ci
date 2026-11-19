//! Time budgets that measure the code, not the machine (#582). A budget test asserts on the
//! CPU time its own thread spent, which a busy machine does not inflate: elapsed wall time
//! also counts the time the scheduler gave to other processes, so with eight builds on the box
//! (load 120 to 230) the realm budgets failed for PRs that never touched the realm.
//!
//! Included with `#[path = "support/cpu.rs"] mod cpu;` by each test that needs it (hf-godot's
//! from its own path). On Unix this is the thread's CPU clock; elsewhere (Windows) there is no
//! such clock without another dependency, and it is wall time, as before.
#![allow(dead_code)]

use std::time::Duration;

/// CPU time the calling thread has spent so far.
#[cfg(unix)]
pub fn thread_cpu() -> Duration {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes one timespec through a valid pointer.
    let r = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    assert_eq!(r, 0, "clock_gettime(CLOCK_THREAD_CPUTIME_ID)");
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

#[cfg(not(unix))]
pub fn thread_cpu() -> Duration {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed()
}

/// A stopwatch on the calling thread's CPU clock; it also keeps the wall time, for the report.
pub struct CpuTimer(Duration, std::time::Instant);

impl CpuTimer {
    pub fn start() -> CpuTimer {
        CpuTimer(thread_cpu(), std::time::Instant::now())
    }
    pub fn elapsed(&self) -> Duration {
        thread_cpu().saturating_sub(self.0)
    }
    /// Prints the CPU and wall time so far (shown by `--no-capture`), and returns the CPU time.
    pub fn report(&self, what: &str) -> Duration {
        let cpu = self.elapsed();
        eprintln!("{what}: {cpu:.2?} of CPU, {:.2?} wall", self.1.elapsed());
        cpu
    }
}

/// Run `f` with the sim's parallel parts on this thread alone (a one-thread rayon pool, which
/// `hf_sim::par` uses when called inside one), so the calling thread's CPU clock counts all the
/// work. The result is the same at any thread count (par.rs, tests/threads.rs).
pub fn on_this_thread<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a one-thread pool")
        .install(f)
}

/// The machine's one-minute load average per core, where the system reports it (Linux).
pub fn load_per_core() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/loadavg").ok()?;
    let one: f64 = s.split_whitespace().next()?.parse().ok()?;
    let cores = std::thread::available_parallelism().ok()?.get() as f64;
    Some(one / cores)
}

/// Above this load per core a wall-clock budget measures the machine, not the code.
pub const BUSY_LOAD_PER_CORE: f64 = 1.0;

/// For a budget that has to be wall time (a call that must return at once even if it waits
/// on a lock): true, with a line saying so, when the machine is too busy for the measurement
/// to mean anything; the test then skips its timing assertion, never loosens it.
pub fn too_busy_for_wall_time(test: &str) -> bool {
    match load_per_core() {
        Some(l) if l > BUSY_LOAD_PER_CORE => {
            eprintln!(
                "{test}: wall-clock budget not checked, load {l:.2} per core is over {BUSY_LOAD_PER_CORE} (#582)"
            );
            true
        }
        _ => false,
    }
}
