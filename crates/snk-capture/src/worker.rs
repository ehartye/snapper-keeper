use crate::Result;
use std::sync::Arc;
use tauri::async_runtime::Mutex;

#[derive(Default)]
pub struct CaptureWorker {
    gate: Arc<Mutex<()>>,
}

impl CaptureWorker {
    pub async fn run<T, F>(&self, job: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        let guard = self.gate.clone().lock_owned().await;
        tauri::async_runtime::spawn_blocking(move || {
            // The blocking job outlives a cancelled IPC future. It must own
            // serialization until persistence, events and window restoration finish.
            let _guard = guard;
            #[cfg(target_os = "macos")]
            {
                // Blocking pool threads have no Cocoa run loop to drain temporary
                // Objective-C objects at the end of each native capture job.
                objc2::rc::autoreleasepool(|_| job())
            }
            #[cfg(not(target_os = "macos"))]
            {
                job()
            }
        })
        .await
        .map_err(|error| crate::CaptureError::Os {
            message: format!("capture worker: {error}"),
        })?
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
