use super::*;
use crate::window_hider::{WindowManager, WindowVisibilityGuard};
use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn poll_once<T>(future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
    future.poll(&mut Context::from_waker(&Waker::from(Arc::new(NoopWake))))
}

#[derive(Clone, Default)]
struct Windows(Arc<Mutex<Vec<&'static str>>>);

impl WindowManager for Windows {
    fn list_own_windows(&self) -> Vec<(String, bool)> {
        vec![("library".into(), true)]
    }

    fn hide(&self, _: &str) {
        self.0.lock().unwrap().push("hide");
    }

    fn show(&self, _: &str) {
        self.0.lock().unwrap().push("show");
    }
}

#[test]
fn capture_job_runs_off_the_calling_thread() {
    let caller = std::thread::current().id();
    let actual = tauri::async_runtime::block_on(
        CaptureWorker::default().run(|| Ok(std::thread::current().id())),
    )
    .unwrap();
    assert_ne!(
        caller, actual,
        "native capture must not block the IPC caller"
    );
}

#[test]
fn cancellation_keeps_the_running_lifecycle_serialized() {
    let worker = CaptureWorker::default();
    let windows = Windows::default();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first_windows = windows.clone();
    let mut first = Box::pin(worker.run(move || {
        {
            let _guard = WindowVisibilityGuard::hide_all(&first_windows, &[]);
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(TIMEOUT).unwrap();
            first_windows.0.lock().unwrap().push("persist");
        }
        first_windows.0.lock().unwrap().push("emit");
        Ok(())
    }));
    assert!(poll_once(first.as_mut()).is_pending());
    started_rx.recv_timeout(TIMEOUT).unwrap();

    let cancelled_windows = windows.clone();
    let mut cancelled_waiter = Box::pin(worker.run(move || {
        cancelled_windows
            .0
            .lock()
            .unwrap()
            .push("cancelled-job-ran");
        Ok(())
    }));
    assert!(poll_once(cancelled_waiter.as_mut()).is_pending());
    drop(cancelled_waiter);
    drop(first);
    assert!(
        worker.gate.try_lock().is_err(),
        "running job still owns serialization"
    );

    let next_windows = windows.clone();
    let mut next = Box::pin(worker.run(move || {
        let _guard = WindowVisibilityGuard::hide_all(&next_windows, &[]);
        Ok(())
    }));
    assert!(poll_once(next.as_mut()).is_pending());
    release_tx.send(()).unwrap();
    tauri::async_runtime::block_on(next).unwrap();
    assert_eq!(
        *windows.0.lock().unwrap(),
        ["hide", "persist", "show", "emit", "hide", "show"],
        "cancelled IPC futures must leave persistence, event emission and restoration owned by the running job"
    );
}

#[test]
fn error_and_panic_restore_windows_and_allow_the_next_job() {
    let worker = CaptureWorker::default();
    let windows = Windows::default();
    for panic in [false, true] {
        let job_windows = windows.clone();
        let result: Result<()> = tauri::async_runtime::block_on(worker.run(move || {
            let _guard = WindowVisibilityGuard::hide_all(&job_windows, &[]);
            if panic {
                panic!("native capture panicked");
            }
            Err(crate::CaptureError::NoMonitors)
        }));
        if panic {
            assert!(matches!(result, Err(crate::CaptureError::Os { .. })));
        } else {
            assert!(matches!(result, Err(crate::CaptureError::NoMonitors)));
        }
        assert_eq!(
            tauri::async_runtime::block_on(worker.run(|| Ok(42))).unwrap(),
            42
        );
    }
    assert_eq!(*windows.0.lock().unwrap(), ["hide", "show", "hide", "show"]);
}
