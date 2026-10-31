//! Scheduling priority for the threads controller input passes through. A game that keeps every
//! CPU busy otherwise makes each press wait for a time slice, a millisecond or more, and the
//! daemon's own work is tiny, so running it ahead of the game costs the game next to nothing.
//!
//! On Linux the threads get a lower nice level (a bigger share of the CPU) and a short time slice
//! (EEVDF, Linux 6.12 and later: one that wakes up takes the CPU sooner). That still leaves a
//! busy machine's presses waiting up to a scheduler tick now and then, so with the App setting on
//! (the default) they also run in the real-time class, which takes the CPU at once.

use std::{
    cell::Cell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use crate::monitor::log;

/// Whether input threads should run in the real-time class (the App setting).
static REALTIME: AtomicBool = AtomicBool::new(true);
/// Bumped whenever [`REALTIME`] changes, so each thread applies it once.
static GENERATION: AtomicU32 = AtomicU32::new(0);
/// The last generation logged, so threads applying the same one say it once.
static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);

thread_local! {
    static APPLIED: Cell<Option<u32>> = const { Cell::new(None) };
}

/// Sets whether input threads run in the real-time class. They take it up in [`keep_up`].
pub fn set_realtime(on: bool) {
    if REALTIME.swap(on, Ordering::Relaxed) != on {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
}

/// Called by each thread controller input passes through, as it starts and on every turn of its
/// loop: applies the priority asked for, the first time and whenever it changes. Otherwise it
/// costs a load and a compare.
pub fn keep_up() {
    let generation = GENERATION.load(Ordering::Relaxed);
    if APPLIED.get() == Some(generation) {
        return;
    }
    APPLIED.set(Some(generation));
    let raised = os::raise(REALTIME.load(Ordering::Relaxed));
    if LOGGED.swap(generation, Ordering::Relaxed) != generation {
        match raised {
            Ok(how) => log!("input threads: {how}"),
            Err(why) => log!("input threads run at normal priority: {why}"),
        }
    }
}

#[cfg(target_os = "linux")]
mod os {
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    use crate::monitor::log;

    /// The nice level asked for. rtkit's default floor, which it lowers any thread of the
    /// session to without asking.
    const NICE: i32 = -15;
    /// The time slice asked for, in nanoseconds: the shortest the kernel allows. The threads
    /// block again within microseconds, so it never runs out.
    pub(super) const SLICE_NS: u64 = 100_000;
    /// The real-time priority: within rtkit's default ceiling (20), and below PipeWire's audio
    /// threads, which sound needs more than input does.
    const RT_PRIORITY: u32 = 10;
    /// How long, in microseconds, a real-time thread may keep the CPU without blocking: at the
    /// soft limit it is moved back to normal scheduling (`on_rt_overrun`); the hard limit, where
    /// the kernel would end the process, is rtkit's ceiling, which it requires.
    const RT_SOFT_US: u64 = 100_000;
    const RT_HARD_US: u64 = 200_000;
    /// `SCHED_FLAG_RESET_ON_FORK`: threads spawned by a raised one start out normal.
    const RESET_ON_FORK: u64 = 0x01;

    /// The threads put in the real-time class, for `on_rt_overrun` to move back.
    static RT_THREADS: [AtomicI32; 8] = [const { AtomicI32::new(0) }; 8];
    /// Set when `on_rt_overrun` moved them back; it stays so until the daemon restarts.
    static OVERRAN: AtomicBool = AtomicBool::new(false);

    /// Lowers the thread's nice level and shortens its slice, then puts it in the real-time class
    /// or takes it out, as `realtime` says. What it can't do it says, and the rest still applies.
    pub(super) fn raise(realtime: bool) -> Result<String, String> {
        // SAFETY: gettid has no preconditions.
        let tid = unsafe { libc::gettid() };
        let mut done = Vec::new();
        let mut failed = Vec::new();
        let mut note = |r: Result<&'static str, String>| match r {
            Ok(what) => done.push(what.to_string()),
            Err(why) => failed.push(why),
        };
        // SAFETY: setpriority only reads its arguments.
        if unsafe { libc::setpriority(libc::PRIO_PROCESS, tid as libc::id_t, NICE) } == 0 {
            note(Ok("nice level set directly"));
        } else {
            note(rtkit(|rtkit| {
                let floor: i32 = rtkit.get_property("MinNiceLevel").unwrap_or(NICE);
                rtkit.call::<_, _, ()>("MakeThreadHighPriority", &(tid as u64, NICE.max(floor)))
            })
            .map(|()| "nice level set by rtkit")
            .map_err(|e| format!("rtkit refused a nice level ({e})")));
        }
        note(set_attr(tid, |attr| attr.sched_runtime = SLICE_NS).map(|()| "short time slice").map_err(|e| format!("no short time slice ({e})")));
        if realtime && OVERRAN.load(Ordering::Relaxed) {
            note(Err("not real-time: it kept the CPU too long once, so it stays out until the daemon restarts".into()));
        } else if realtime {
            note(make_realtime(tid));
        } else if leave_realtime(tid) {
            note(Ok("out of the real-time class"));
        }
        match (done.is_empty(), failed.is_empty()) {
            (true, _) => Err(failed.join("; ")),
            (false, true) => Ok(done.join(", ")),
            (false, false) => Ok(format!("{}; but {}", done.join(", "), failed.join("; "))),
        }
    }

    /// Puts thread `tid` in the real-time class, itself where it may (CAP_SYS_NICE, RLIMIT_RTPRIO),
    /// else through rtkit, after setting up the guard against it keeping the CPU too long.
    fn make_realtime(tid: libc::pid_t) -> Result<&'static str, String> {
        guard_rt_time()?;
        let slot = RT_THREADS.iter().find(|s| s.load(Ordering::Relaxed) == tid).or_else(|| {
            RT_THREADS.iter().find(|s| s.compare_exchange(0, tid, Ordering::Relaxed, Ordering::Relaxed).is_ok())
        });
        if slot.is_none() {
            return Err("not real-time: too many threads asked".into());
        }
        let direct = set_attr(tid, |attr| {
            attr.sched_policy = libc::SCHED_FIFO as u32;
            attr.sched_priority = RT_PRIORITY;
            attr.sched_flags |= RESET_ON_FORK;
        });
        if direct.is_ok() {
            return Ok("real-time class, set directly");
        }
        rtkit(|rtkit| rtkit.call::<_, _, ()>("MakeThreadRealtime", &(tid as u64, RT_PRIORITY)))
            .map(|()| "real-time class, set by rtkit")
            .map_err(|e| format!("not real-time: rtkit refused ({e})"))
    }

    /// Takes thread `tid` out of the real-time class, if it is in it. Whether it was.
    fn leave_realtime(tid: libc::pid_t) -> bool {
        let Some(slot) = RT_THREADS.iter().find(|s| s.load(Ordering::Relaxed) == tid) else { return false };
        slot.store(0, Ordering::Relaxed);
        to_normal(tid);
        true
    }

    /// Moves thread `tid` to normal scheduling, keeping its nice level, slice and flags.
    /// Async-signal-safe: system calls only.
    fn to_normal(tid: libc::pid_t) {
        // A real-time thread keeps its nice level, but sched_getattr reports it as 0.
        // SAFETY: getpriority only reads its arguments.
        let nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, tid as libc::id_t) };
        let _ = set_attr(tid, |attr| {
            attr.sched_nice = nice;
            attr.sched_policy = libc::SCHED_OTHER as u32;
            attr.sched_priority = 0;
            attr.sched_runtime = SLICE_NS;
        });
    }

    /// Changes thread `tid`'s scheduling as `change` says, keeping the rest of it (such as the
    /// reset-on-fork flag rtkit sets, which only a privileged caller may clear). Kernels before
    /// 6.12 ignore a fair thread's slice.
    fn set_attr(tid: libc::pid_t, change: impl FnOnce(&mut libc::sched_attr)) -> std::io::Result<()> {
        let size = std::mem::size_of::<libc::sched_attr>() as u32;
        // SAFETY: an all-zero sched_attr is a valid value of it.
        let mut attr: libc::sched_attr = unsafe { std::mem::zeroed() };
        // SAFETY: `attr` is alive for the call, which writes at most `size` bytes to it.
        if unsafe { libc::syscall(libc::SYS_sched_getattr, tid, &mut attr, size, 0) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        attr.size = size;
        change(&mut attr);
        // SAFETY: `attr` is a valid sched_attr of the size it states, alive for the call.
        if unsafe { libc::syscall(libc::SYS_sched_setattr, tid, &attr, 0) } == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
    }

    /// Limits how long a real-time thread may run without blocking (rtkit requires a limit), and
    /// handles the soft limit by moving the threads back to normal scheduling. Once per process.
    fn guard_rt_time() -> Result<(), String> {
        static GUARDED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
        GUARDED
            .get_or_init(|| {
                let mut limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
                // SAFETY: `limit` is alive for the call.
                unsafe { libc::getrlimit(libc::RLIMIT_RTTIME, &mut limit) };
                let hard = limit.rlim_max.min(RT_HARD_US);
                let limit = libc::rlimit { rlim_cur: RT_SOFT_US.min(hard), rlim_max: hard };
                // SAFETY: as above; lowering limits needs no rights.
                if unsafe { libc::setrlimit(libc::RLIMIT_RTTIME, &limit) } != 0 {
                    return Err(format!("not real-time: no limit on its CPU time ({})", std::io::Error::last_os_error()));
                }
                // SAFETY: a zeroed sigaction with a handler and an empty mask is valid; the handler
                // only makes system calls and stores atomics, which is async-signal-safe.
                let installed = unsafe {
                    let mut action: libc::sigaction = std::mem::zeroed();
                    action.sa_sigaction = on_rt_overrun as *const () as libc::sighandler_t;
                    action.sa_flags = libc::SA_RESTART;
                    libc::sigemptyset(&mut action.sa_mask);
                    libc::sigaction(libc::SIGXCPU, &action, std::ptr::null_mut())
                };
                if installed != 0 {
                    return Err(format!("not real-time: no guard on its CPU time ({})", std::io::Error::last_os_error()));
                }
                Ok(())
            })
            .clone()
    }

    /// SIGXCPU: a real-time thread kept the CPU past the soft limit. Moves every one back to
    /// normal scheduling before the hard limit ends the process.
    extern "C" fn on_rt_overrun(_: libc::c_int) {
        for slot in &RT_THREADS {
            let tid = slot.swap(0, Ordering::Relaxed);
            if tid != 0 {
                to_normal(tid);
            }
        }
        OVERRAN.store(true, Ordering::Relaxed);
    }

    /// Whether a real-time thread was moved back since this was last asked, for the log.
    pub(super) fn take_overrun() -> bool {
        static SAID: AtomicBool = AtomicBool::new(false);
        OVERRAN.load(Ordering::Relaxed) && !SAID.swap(true, Ordering::Relaxed)
    }

    fn rtkit<T>(call: impl FnOnce(&zbus::blocking::Proxy) -> zbus::Result<T>) -> zbus::Result<T> {
        let bus = zbus::blocking::Connection::system()?;
        let rtkit = zbus::blocking::Proxy::new(&bus, "org.freedesktop.RealtimeKit1", "/org/freedesktop/RealtimeKit1", "org.freedesktop.RealtimeKit1")?;
        let result = call(&rtkit);
        if let Err(e) = &result {
            log!("rtkit: {e}");
        }
        result
    }
}

/// Logs it once if an input thread had to leave the real-time class for keeping the CPU too long.
/// For the main loop to call now and then.
pub fn check() {
    if os::take_overrun() {
        log!("input threads: back to normal priority, after keeping the CPU over 100 ms in the real-time class");
    }
}

#[cfg(test)]
mod tests {
    /// Asks the system for real, so it is opt-in: `cargo test raises_ -- --ignored`. It checks
    /// what it was granted: on a desktop with rtkit, all of it, without rights of its own.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn raises_the_calling_thread_and_lowers_it_again() {
        let attr = || {
            // SAFETY: an all-zero sched_attr is valid, and the call writes at most `size` bytes.
            let mut attr: libc::sched_attr = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of::<libc::sched_attr>() as u32;
            // SAFETY: as above; `attr` is alive for the call.
            assert_eq!(unsafe { libc::syscall(libc::SYS_sched_getattr, 0, &mut attr, size, 0) }, 0);
            attr
        };
        std::thread::spawn(move || {
            let said = super::os::raise(true).unwrap();
            assert!(!said.contains("no short"), "{said}");
            let raised = attr();
            // SAFETY: getpriority only reads its arguments. (sched_getattr says 0 for a
            // real-time thread's nice level, which it keeps all the same.)
            let nice = || unsafe { libc::getpriority(libc::PRIO_PROCESS, libc::gettid() as libc::id_t) };
            let raised_nice = nice();
            if said.contains("nice level set") {
                assert!(raised_nice < 0, "{said}: nice level {raised_nice}");
            }
            if said.contains("real-time class") {
                // rtkit picks round-robin; set directly it is first-in first-out. Either is real-time.
                assert!([libc::SCHED_FIFO as u32, libc::SCHED_RR as u32].contains(&raised.sched_policy), "{said}: policy {}", raised.sched_policy);
            } else {
                assert_eq!(raised.sched_runtime, super::os::SLICE_NS, "{said}");
            }
            let said = super::os::raise(false).unwrap();
            let lowered = attr();
            assert_eq!((lowered.sched_policy, lowered.sched_runtime), (libc::SCHED_OTHER as u32, super::os::SLICE_NS), "{said}");
            assert_eq!(nice(), raised_nice, "the nice level stays");
        })
        .join()
        .unwrap();
    }

    /// A real-time thread that keeps the CPU past the soft limit is moved back to normal
    /// scheduling, and the process lives on past the hard limit. Opt-in like the one above; it
    /// keeps one CPU busy for a third of a second. Should the guard fail, the kernel ends the
    /// test binary.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn a_real_time_thread_that_keeps_the_cpu_is_moved_back() {
        std::thread::spawn(|| {
            let said = super::os::raise(true).unwrap();
            // Nothing to check where the system grants no real-time class.
            if !said.contains("real-time class") {
                return;
            }
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_millis(300) {
                std::hint::spin_loop();
            }
            // SAFETY: sched_getscheduler only reads its argument.
            assert_eq!(unsafe { libc::sched_getscheduler(0) } & !libc::SCHED_RESET_ON_FORK, libc::SCHED_OTHER);
            assert!(super::os::take_overrun(), "the overrun is reported once");
            assert!(!super::os::take_overrun());
        })
        .join()
        .unwrap();
    }
}
