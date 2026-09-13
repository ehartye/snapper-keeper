use crate::Result;
use std::sync::Arc;
use tauri::async_runtime::Mutex;

#[derive(Default)]
pub struct CaptureWorker {
    gate: Arc<Mutex<crate::preview::PreviewSession>>,
}

impl CaptureWorker {
    pub async fn run<T, F>(&self, job: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        self.run_with_preview(move |_| job()).await
    }

    pub async fn run_with_preview<T, F>(&self, job: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut crate::preview::PreviewSession) -> Result<T> + Send + 'static,
    {
        let mut guard = self.gate.clone().lock_owned().await;
        tauri::async_runtime::spawn_blocking(move || {
            // The blocking job outlives a cancelled IPC future. It must own
            // serialization until persistence, events and window restoration finish.
            #[cfg(target_os = "macos")]
            {
                // Blocking pool threads have no Cocoa run loop to drain temporary
                // Objective-C objects at the end of each native capture job.
                objc2::rc::autoreleasepool(|_| job(&mut guard))
            }
            #[cfg(not(target_os = "macos"))]
            {
                job(&mut guard)
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
