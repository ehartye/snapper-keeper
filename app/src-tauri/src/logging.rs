//! File-based logging + panic hook setup.
//!
//! Per [#25](https://github.com/ehartye/snapper-keeper/issues/25):
//! - Daily-rotating general log under `app_log_dir`, kept 30 days.
//! - Panic hook writes full crash dumps to `crashes/<RFC3339-timestamp>.log`.
//! - Stdout layer preserved for dev convenience.
//!
//! Per [#34](https://github.com/ehartye/snapper-keeper/issues/34) and
//! [#35](https://github.com/ehartye/snapper-keeper/issues/35):
//! - Separate append-only, never-rotated `security-events.log` for low-
//!   frequency forensic events (updater sig failures, capability denials,
//!   `paste_item` invocations, autostart toggles, panic captures,
//!   hard-deletes). Routes events tagged with `target = "snk::security"`.
//!
//! The general log file gives post-incident forensics for everything from
//! XSS chains to silent watcher death to updater errors. Before #25, the
//! `windows_subsystem = "windows"` build hides stdout, so without file
//! logging the field forensics surface was literally empty.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Builder as RollingBuilder;
use tracing_subscriber::{
    filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

/// Number of daily log files to keep before auto-deleting the oldest.
const LOG_RETENTION_DAYS: usize = 30;

/// Filename prefix for the rotating general log.
const LOG_PREFIX: &str = "snapper-keeper";

/// Subdirectory name under `app_log_dir` for crash dumps.
const CRASH_SUBDIR: &str = "crashes";

/// Filename for the never-rotated security events log. Single file,
/// append-only, no rotation. Security events are low-frequency, so the
/// file's growth is bounded by user behavior rather than time.
const SECURITY_LOG_FILENAME: &str = "security-events.log";

/// Tracing target string that routes events into the security-events
/// log. Use as `tracing::info!(target: SECURITY_LOG_TARGET, ...)`.
pub const SECURITY_LOG_TARGET: &str = "snk::security";

/// Handle to the file-appender background workers. Both the general
/// and security log writers run on background threads; their
/// WorkerGuards MUST be held for the program's lifetime. Returned from
/// [`init`] so `main` keeps them alive.
pub struct LoggingHandle {
    _general_guard: WorkerGuard,
    _security_guard: WorkerGuard,
}

/// Initialize the tracing subscriber with stdout + daily-rotating
/// general file + never-rotated security file layers, and register a
/// panic hook that captures crash dumps to disk.
///
/// `log_dir` is typically `app.path().app_log_dir().unwrap()` from the
/// Tauri AppHandle. The directory is created if missing.
///
/// Returns a [`LoggingHandle`] that must be kept alive for the program's
/// duration so the background log-writer threads stay running.
pub fn init(log_dir: &Path) -> Result<LoggingHandle, std::io::Error> {
    std::fs::create_dir_all(log_dir)?;
    std::fs::create_dir_all(log_dir.join(CRASH_SUBDIR))?;

    // General log: daily-rotating, 30-day retention.
    let general_appender = RollingBuilder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_PREFIX)
        .filename_suffix("log")
        .max_log_files(LOG_RETENTION_DAYS)
        .build(log_dir)
        .map_err(std::io::Error::other)?;
    let (general_non_blocking, general_guard) = tracing_appender::non_blocking(general_appender);

    // Security log: single file, never-rotated, append-only. Events
    // emitted via `tracing::info!(target: SECURITY_LOG_TARGET, ...)`
    // (or any other level with that target) route here in addition to
    // the general log.
    let security_appender = tracing_appender::rolling::never(log_dir, SECURITY_LOG_FILENAME);
    let (security_non_blocking, security_guard) = tracing_appender::non_blocking(security_appender);

    let env_filter =
        || EnvFilter::try_from_env("SNK_LOG").unwrap_or_else(|_| EnvFilter::new("info,snk=debug"));

    // Filter that matches events emitted with target = SECURITY_LOG_TARGET.
    let security_only = filter_fn(|meta| meta.target() == SECURITY_LOG_TARGET);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(env_filter()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(general_non_blocking)
                .with_ansi(false)
                .with_filter(env_filter()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(security_non_blocking)
                .with_ansi(false)
                .with_filter(security_only),
        )
        .init();

    install_panic_hook(log_dir.join(CRASH_SUBDIR));

    tracing::info!(
        log_dir = %log_dir.display(),
        retention_days = LOG_RETENTION_DAYS,
        "logging initialized"
    );

    Ok(LoggingHandle {
        _general_guard: general_guard,
        _security_guard: security_guard,
    })
}

/// Install a global panic hook that writes the panic + backtrace to a
/// timestamped file under `crash_dir`. The previous default hook (which
/// writes to stderr) is preserved as well so any attached terminal still
/// sees the panic.
fn install_panic_hook(crash_dir: PathBuf) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // First: write the dump to disk. Do this before delegating to the
        // default hook in case the default hook decides to abort.
        let _ = write_crash_dump(&crash_dir, info);
        default_hook(info);
    }));
}

fn write_crash_dump(crash_dir: &Path, info: &std::panic::PanicHookInfo<'_>) -> std::io::Result<()> {
    use std::io::Write;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S%.3fZ");
    let path = crash_dir.join(format!("crash-{timestamp}.log"));
    let mut f = std::fs::File::create(&path)?;

    writeln!(f, "# snapper-keeper crash dump")?;
    writeln!(f, "Timestamp (UTC): {timestamp}")?;
    writeln!(f, "App version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(f, "OS: {} {}", std::env::consts::OS, std::env::consts::ARCH)?;
    writeln!(f)?;
    writeln!(f, "## Panic")?;
    writeln!(f, "{info}")?;
    writeln!(f)?;
    writeln!(f, "## Backtrace")?;
    let backtrace = std::backtrace::Backtrace::force_capture();
    writeln!(f, "{backtrace}")?;

    // Also emit a tracing event so the panic shows up in today's regular
    // log file. The default hook will print to stderr.
    tracing::error!(
        crash_dump = %path.display(),
        panic = %info,
        "panic captured"
    );

    // And route a security event so the panic appears in the
    // never-rotated security log (per #34 enumerated event list).
    tracing::error!(
        target: SECURITY_LOG_TARGET,
        event_type = "panic_captured",
        crash_dump = %path.display(),
        panic = %info,
        "panic captured"
    );

    Ok(())
}
