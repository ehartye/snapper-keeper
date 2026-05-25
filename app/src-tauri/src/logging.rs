//! File-based logging + panic hook setup.
//!
//! Per [#25](https://github.com/ehartye/snapper-keeper/issues/25):
//! - Daily-rotating general log under `app_log_dir`, kept 30 days.
//! - Panic hook writes full crash dumps to `crashes/<RFC3339-timestamp>.log`.
//! - Stdout layer preserved for dev convenience.
//!
//! The general log file gives post-incident forensics for everything from
//! XSS chains to silent watcher death to updater errors. Before #25, the
//! `windows_subsystem = "windows"` build hides stdout, so without file
//! logging the field forensics surface was literally empty.

use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Builder as RollingBuilder;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Number of daily log files to keep before auto-deleting the oldest.
const LOG_RETENTION_DAYS: usize = 30;

/// Filename prefix for the rotating general log.
const LOG_PREFIX: &str = "snapper-keeper";

/// Subdirectory name under `app_log_dir` for crash dumps.
const CRASH_SUBDIR: &str = "crashes";

/// Handle to the file-appender's background worker. MUST be held for the
/// program's lifetime; dropping it flushes pending log lines and stops the
/// background thread. Returned from [`init`] so `main` keeps it alive.
pub struct LoggingHandle {
    _guard: WorkerGuard,
}

/// Initialize the tracing subscriber with stdout + daily-rotating file
/// layers, and register a panic hook that captures crash dumps to disk.
///
/// `log_dir` is typically `app.path().app_log_dir().unwrap()` from the
/// Tauri AppHandle. The directory is created if missing.
///
/// Returns a [`LoggingHandle`] that must be kept alive for the program's
/// duration so the background log-writer thread stays running.
pub fn init(log_dir: &Path) -> Result<LoggingHandle, std::io::Error> {
    std::fs::create_dir_all(log_dir)?;
    std::fs::create_dir_all(log_dir.join(CRASH_SUBDIR))?;

    let file_appender = RollingBuilder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_PREFIX)
        .filename_suffix("log")
        .max_log_files(LOG_RETENTION_DAYS)
        .build(log_dir)
        .map_err(std::io::Error::other)?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = || {
        EnvFilter::try_from_env("SNK_LOG").unwrap_or_else(|_| EnvFilter::new("info,snk=debug"))
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(env_filter()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(env_filter()),
        )
        .init();

    install_panic_hook(log_dir.join(CRASH_SUBDIR));

    tracing::info!(
        log_dir = %log_dir.display(),
        retention_days = LOG_RETENTION_DAYS,
        "logging initialized"
    );

    Ok(LoggingHandle { _guard: guard })
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

fn write_crash_dump(
    crash_dir: &Path,
    info: &std::panic::PanicHookInfo<'_>,
) -> std::io::Result<()> {
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

    Ok(())
}
