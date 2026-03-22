/// Task 26.1 / D.1 — Multi-Worker Event Loop
///
/// A lightweight task scheduler backed by a configurable thread pool.
/// When enabled via `--experimental-event-loop`, `async_run()` submits tasks
/// to this pool instead of spawning one OS thread per task.
///
/// # Guarantees
/// - N worker threads (default = logical CPUs, min 2, max 64).
/// - Tasks execute concurrently across workers.
/// - Falls back gracefully to OS-thread-per-task when disabled.
///
/// # Enabling
/// ```text
/// txtcode run --experimental-event-loop script.tc
/// txtcode run --experimental-event-loop --event-loop-workers 8 script.tc
/// ```

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Opaque task type submitted to the event loop.
type Task = Box<dyn FnOnce() + Send + 'static>;

/// Internal thread-pool task runner.
struct TaskQueue {
    sender: std::sync::mpsc::SyncSender<Task>,
}

impl TaskQueue {
    /// Start a thread pool with `workers` concurrent worker threads.
    fn start(workers: usize) -> Self {
        let workers = workers.clamp(1, 64);
        // Bounded channel — applies back-pressure when workers fall behind.
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Task>(4096);
        let receiver = Arc::new(Mutex::new(receiver));

        for i in 0..workers {
            let rx = Arc::clone(&receiver);
            thread::Builder::new()
                .name(format!("txtcode-worker-{}", i))
                .spawn(move || {
                    while let Ok(task) = {
                        let guard = rx.lock().unwrap();
                        guard.recv()
                    } {
                        WORKERS_ACTIVE.fetch_add(1, Ordering::Relaxed);
                        task();
                        WORKERS_ACTIVE.fetch_sub(1, Ordering::Relaxed);
                        // Saturating decrement: prevent wrap-around when disable_for_test()
                        // resets ACTIVE_TASKS to 0 while a worker is still completing a task.
                        let _ = ACTIVE_TASKS.fetch_update(
                            Ordering::Relaxed, Ordering::Relaxed,
                            |v| Some(v.saturating_sub(1)),
                        );
                    }
                })
                .expect("failed to spawn event loop worker thread");
        }
        Self { sender }
    }

    fn submit(&self, task: Task) -> bool {
        self.sender.send(task).is_ok()
    }
}

// ── Global state ──────────────────────────────────────────────────────────────

static EVENT_LOOP_ENABLED: AtomicBool = AtomicBool::new(false);

/// Count of tasks submitted since the event loop was enabled (for diagnostics).
pub static TASKS_SUBMITTED: AtomicI64 = AtomicI64::new(0);

/// Count of tasks currently executing across worker threads.
pub static WORKERS_ACTIVE: AtomicI64 = AtomicI64::new(0);

/// Configured worker count (0 = not yet set; default resolved at enable time).
static WORKER_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Maximum number of tasks that may be active (running + queued) concurrently.
/// 0 means "unlimited". Default = 64.
static MAX_CONCURRENT_TASKS: AtomicUsize = AtomicUsize::new(64);

/// Count of tasks currently active (submitted but not yet finished).
pub static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);

lazy_static::lazy_static! {
    static ref TASK_QUEUE: Mutex<Option<TaskQueue>> = Mutex::new(None);
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Configure the number of worker threads.  Must be called before `enable()`.
/// Values are clamped to [1, 64].  Default is `available_parallelism()` (min 2).
pub fn set_worker_count(n: usize) {
    WORKER_COUNT.store(n.clamp(1, 64), Ordering::SeqCst);
}

/// Return the currently configured worker count (0 if not yet configured).
pub fn worker_count() -> usize {
    WORKER_COUNT.load(Ordering::SeqCst)
}

/// Set the maximum number of concurrently active async tasks (default 64).
/// Use 0 for unlimited. Values above 1024 are clamped to 1024.
pub fn set_max_concurrent_tasks(n: usize) {
    MAX_CONCURRENT_TASKS.store(if n == 0 { 0 } else { n.min(1024) }, Ordering::SeqCst);
}

/// Return the current concurrency cap (0 = unlimited).
pub fn max_concurrent_tasks() -> usize {
    MAX_CONCURRENT_TASKS.load(Ordering::SeqCst)
}

/// Enable the event loop.  Starts the thread pool on first call.
/// Subsequent calls are no-ops.
pub fn enable() {
    let mut guard = TASK_QUEUE.lock().unwrap();
    if guard.is_none() {
        let n = {
            let configured = WORKER_COUNT.load(Ordering::SeqCst);
            if configured > 0 {
                configured
            } else {
                // Default: logical CPUs, at least 2
                let cpus = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(2);
                cpus.max(2)
            }
        };
        *guard = Some(TaskQueue::start(n));
    }
    EVENT_LOOP_ENABLED.store(true, Ordering::SeqCst);
}

/// Returns `true` when the event loop is active.
pub fn is_enabled() -> bool {
    EVENT_LOOP_ENABLED.load(Ordering::SeqCst)
}

/// Submit a task to the event loop.  Returns `true` on success.
///
/// Returns `false` when:
/// - the event loop is disabled (caller falls back to thread-per-task), or
/// - the concurrency cap has been reached (`max_concurrent_tasks() > 0` and
///   `ACTIVE_TASKS >= max_concurrent_tasks()`).
///
/// When the concurrency cap is reached the caller should return an `E0053`
/// error to the script rather than silently dropping the task.
pub fn submit(task: Box<dyn FnOnce() + Send + 'static>) -> bool {
    if !is_enabled() {
        return false;
    }

    // Enforce concurrency cap (0 = unlimited).
    let cap = MAX_CONCURRENT_TASKS.load(Ordering::SeqCst);
    if cap > 0 {
        // Try to reserve a slot via compare-and-swap loop.
        loop {
            let current = ACTIVE_TASKS.load(Ordering::SeqCst);
            if current >= cap {
                return false; // cap reached — caller must back-pressure
            }
            if ACTIVE_TASKS.compare_exchange(
                current, current + 1, Ordering::SeqCst, Ordering::SeqCst
            ).is_ok() {
                break; // slot reserved
            }
            // Another thread raced us — retry
        }
    }

    let guard = TASK_QUEUE.lock().unwrap();
    if let Some(q) = guard.as_ref() {
        let ok = q.submit(task);
        if ok {
            TASKS_SUBMITTED.fetch_add(1, Ordering::Relaxed);
        } else if cap > 0 {
            // Channel send failed — release the slot we reserved above.
            ACTIVE_TASKS.fetch_sub(1, Ordering::Relaxed);
        }
        ok
    } else {
        if cap > 0 {
            ACTIVE_TASKS.fetch_sub(1, Ordering::Relaxed);
        }
        false
    }
}

/// Disable the event loop flag (for testing isolation).
/// Does NOT stop worker threads — they stay parked waiting on the channel.
pub fn disable_for_test() {
    EVENT_LOOP_ENABLED.store(false, Ordering::SeqCst);
    TASKS_SUBMITTED.store(0, Ordering::Relaxed);
    ACTIVE_TASKS.store(0, Ordering::Relaxed);
}
