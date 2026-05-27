//! Per-OS event-driven clipboard observation. macOS uses polling
//! (handled directly in `watcher.rs`); Windows uses
//! AddClipboardFormatListener + WM_CLIPBOARDUPDATE.

#[cfg(target_os = "windows")]
pub mod windows {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, OnceLock};

    use arboard::Clipboard;
    use snk_library::Db;
    use tracing::{debug, error};
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassExW,
        TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
        WNDCLASSEXW,
    };

    use crate::sensitivity::OsProbe;
    use crate::source_app;
    use crate::watcher::{worker_step, ClipboardEvent, WatcherState};

    /// Storage handed to the window-procedure callback via a process-global
    /// OnceLock. Wrapping in Mutex makes interior mutation safe even though
    /// in practice the listener thread is the only accessor.
    /// OsProbe is omitted (it's a ZST; we construct &OsProbe at the call site).
    struct WatcherCtx {
        db: Arc<Db>,
        library_root: PathBuf,
        state: WatcherState,
        clipboard: Clipboard,
    }

    // arboard::Clipboard on Windows is a ZST (pub(crate) struct Clipboard(())),
    // so WatcherCtx is Send+Sync automatically. Mutex<WatcherCtx> works.
    static CTX: OnceLock<Mutex<WatcherCtx>> = OnceLock::new();

    pub fn start(db: Arc<Db>, library_root: PathBuf) {
        // Spin up a dedicated thread that owns the message-only window.
        // The watcher must run on this thread because WM_CLIPBOARDUPDATE
        // is dispatched into the thread that owns the listener handle.
        std::thread::Builder::new()
            .name("snk-clipboard-listener".into())
            .spawn(move || {
                let clipboard = match Clipboard::new() {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "failed to open clipboard for watching");
                        return;
                    }
                };
                if CTX
                    .set(Mutex::new(WatcherCtx {
                        db,
                        library_root,
                        state: WatcherState::new(),
                        clipboard,
                    }))
                    .is_err()
                {
                    error!("clipboard watcher CTX already initialized; refusing to start twice");
                    return;
                }
                run_message_loop();
            })
            .expect("spawn snk-clipboard-listener thread");
    }

    fn run_message_loop() {
        // SAFETY: the entire message loop is a sequence of Win32 calls
        // that are documented to be safe when invoked on the thread that
        // owns the message-only window we create here. The window class
        // is keyed by name and is idempotent across processes.
        unsafe {
            let instance = GetModuleHandleW(None).expect("module handle");
            let class_name = w!("SnkClipboardListener");
            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: class_name,
                ..Default::default()
            };
            // Class registration is idempotent; the returned ATOM is
            // unused because we look up the class by name in CreateWindowExW.
            let _atom = RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                w!("snk-clipboard"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )
            .expect("create message-only window");

            if AddClipboardFormatListener(hwnd).is_err() {
                error!("AddClipboardFormatListener failed; falling back to polling thread");
                let _ = RemoveClipboardFormatListener(hwnd);
                // Fallback path: spawn the polling loop. CTX is already
                // initialized on this thread; the polling helper opens
                // its own Clipboard handle and runs in its own thread.
                if let Some(ctx_mtx) = CTX.get() {
                    let ctx = ctx_mtx.lock().expect("CTX mutex poisoned");
                    crate::watcher::start_polling(
                        ctx.db.clone(),
                        ctx.library_root.clone(),
                        std::time::Duration::from_millis(500),
                    );
                }
                return;
            }

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, Some(hwnd), 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            handle_clipboard_update();
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn handle_clipboard_update() {
        // Skip-set check moved into worker_step (content-hash based,
        // see SkipReason::OwnWrite). The WM_CLIPBOARDUPDATE handler
        // proceeds to read+hash the clipboard event; worker_step
        // recognizes any self-emitted content and skips it there.

        let ctx_mtx = CTX
            .get()
            .expect("watcher CTX initialized before clipboard event");
        let mut ctx = ctx_mtx.lock().expect("CTX mutex poisoned");

        // Try text first, then image. arboard reads from the system
        // clipboard, which the WM_CLIPBOARDUPDATE handler can access
        // without explicit OpenClipboard (arboard handles that itself).
        let event = match ctx.clipboard.get_text() {
            Ok(t) if !t.is_empty() => ClipboardEvent::Text(t),
            _ => match ctx.clipboard.get_image() {
                Ok(img) if !img.bytes.is_empty() => {
                    let width = img.width;
                    let height = img.height;
                    ClipboardEvent::Image { bytes: img.bytes.into_owned(), width, height }
                }
                _ => return,
            },
        };

        let source = source_app::current();
        // Split borrow: clone the Arc<Db> and PathBuf so worker_step can
        // hold them by reference without conflicting with &mut ctx.state.
        // OsProbe is a ZST — passing &OsProbe is free and avoids a Copy bound.
        let db = ctx.db.clone();
        let library_root = ctx.library_root.clone();
        let result = worker_step(event, &mut ctx.state, &db, &library_root, &OsProbe, source);
        match result {
            crate::watcher::StepResult::Skipped(reason) => {
                debug!(?reason, "clipboard event skipped");
            }
            crate::watcher::StepResult::Saved { item_id } => {
                debug!(item_id, "clipboard event saved");
            }
            crate::watcher::StepResult::DedupedTo { existing_id } => {
                debug!(existing_id, "clipboard event deduplicated");
            }
        }
    }
}
