use crate::cli::CliOptions;
use crate::get_output_dir;
use chrono::Local;
use clap::Parser;
use eyre::WrapErr;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::cmp::min;
use std::fs::{create_dir_all, File};
use std::io::Error as IoError;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::metadata::LevelFilter;
use tracing::{error, info};
use tracing_subscriber::fmt::format::{FmtSpan, Writer};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, Registry};

static TRACING_GUARD: Mutex<Option<TracingGuard>> = Mutex::new(None);

/// Registers a signal handler that tries to ensure correct termination of logging
/// in the presence of sudden program termination.
pub fn register_signal_handler() -> eyre::Result<()> {
    ctrlc::set_handler(|| {
        error!(target: "dynamecs_app", "Received signal to terminate (for example Ctrl+C). Aborting application...");
        if let Ok(mut opt) = TRACING_GUARD.lock() {
            if let Some(guard) = opt.as_mut() {
                guard.finalize();
            }
        }
        std::process::exit(1);
    })?;
    Ok(())
}

/// Sets up `tracing`.
///
/// Returns a guard that should be kept alive.
/// The guard ensures that all streams, such as compressed gzip for JSON logs, are properly flushed
/// when it goes out of scope.
///
/// TODO: Describe the tracing setup, i.e. log to stdout, file etc.
///
/// ```
/// use dynamecs_app::setup_tracing;
/// use std::error::Error;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let _guard = setup_tracing()?;
///     // do something here. The guard lives until the end of the function
/// }
/// ```
#[must_use]
pub fn setup_tracing() -> eyre::Result<TracingGuard> {
    let cli_options = CliOptions::parse();

    let gz_ext = match cli_options.compress_logs {
        true => ".gz",
        false => "",
    };
    let log_dir = get_output_dir().join("logs");
    let log_file_base_name = "dynamecs_app.log";
    let json_log_file_base_name = "dynamecs_app.jsonlog";
    remove_non_archive_log_files(log_dir.as_ref(), log_file_base_name, json_log_file_base_name)?;
    let log_file_path = log_dir.join(format!("{log_file_base_name}{gz_ext}"));
    let json_log_file_path = log_dir.join(format!("{json_log_file_base_name}{gz_ext}"));

    // Use ISO 8601 / RFC 3339, but replace colons with dots, since colons are
    // not valid in Windows filenames (and awkward on Unix)
    let timestamp = format!("{}", Local::now().format("%+")).replace(":", ".");
    let archive_dir = log_dir.join("archive");
    let archive_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.log{gz_ext}"));
    let archive_json_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.json{gz_ext}"));

    create_dir_all(&log_dir).wrap_err("failed to create log directory")?;
    let log_file = File::create(&log_file_path).wrap_err("failed to create main log file")?;
    let json_log_file = File::create(&json_log_file_path).wrap_err("failed to create json log file")?;
    let mut log_files = vec![log_file];
    let mut json_log_files = vec![json_log_file];

    if cli_options.archive_logs {
        create_dir_all(&archive_dir).wrap_err("failed to create log archive directory")?;
        let archive_log_file = File::create(&archive_log_file_path).wrap_err("failed to create archive log file")?;
        let archive_json_log_file =
            File::create(&archive_json_log_file_path).wrap_err("failed to create archive json log file")?;
        log_files.push(archive_log_file);
        json_log_files.push(archive_json_log_file);
    }

    let mut guard = TracingGuard::new();

    let log_files_writer = MultiWriter::from_writers(log_files);
    let json_files_writer = MultiWriter::from_writers(json_log_files);
    if cli_options.compress_logs {
        let log_gzip_writer = GzipLogWriter::new(log_files_writer);
        let log_writer = Arc::new(MutexWriter::new(log_gzip_writer));
        let json_gzip_writer = GzipLogWriter::new(json_files_writer);
        let json_writer = Arc::new(MutexWriter::new(json_gzip_writer));

        guard.gz_log_file_writer = Some(Arc::clone(&log_writer));
        guard.gz_json_log_file_writer = Some(Arc::clone(&json_writer));

        set_global_tracing_subscriber(
            cli_options.console_log_level,
            cli_options.file_log_level,
            log_writer,
            json_writer,
        )?;
    } else {
        let log_writer = Arc::new(MutexWriter::new(log_files_writer));
        let json_writer = Arc::new(MutexWriter::new(json_files_writer));

        guard.log_file_writer = Some(Arc::clone(&log_writer));
        guard.json_log_file_writer = Some(Arc::clone(&json_writer));

        set_global_tracing_subscriber(
            cli_options.console_log_level,
            cli_options.file_log_level,
            log_writer,
            json_writer,
        )?;
    }

    let working_dir = std::env::current_dir().wrap_err("failed to retrieve current working directory")?;
    info!(target: "dynamecs_app", "Working directory: {}", working_dir.display());
    info!(target: "dynamecs_app", "Logging text to stdout with log level {}", cli_options.console_log_level.to_string());
    info!(target: "dymamecs_app", "Logging text to file {} with log level {}", log_file_path.display(), cli_options.file_log_level);
    info!(target: "dynamecs_app", "Logging JSON to file {} with log level {}", json_log_file_path.display(), cli_options.file_log_level);
    if cli_options.archive_logs {
        info!(target: "dynamecs_app", "Archived log file path:  {}", archive_log_file_path.display());
        info!(target: "dynamecs_app", "Archived JSON log file path: {}", archive_json_log_file_path.display());
    }

    TRACING_GUARD
        .lock()
        .expect("Internal error: Poisoned mutex")
        .replace(guard.clone_private());

    Ok(guard)
}

/// Remove old non-archive log files so that there are no stale logs when toggling log
/// compression.
fn remove_non_archive_log_files(
    directory: &Path,
    log_base_name: &str,
    json_log_base_name: &str,
) -> std::io::Result<()> {
    let names = [
        log_base_name.to_string(),
        json_log_base_name.to_string(),
        format!("{log_base_name}.gz"),
        format!("{json_log_base_name}.gz"),
    ];
    for name in names {
        remove_file_if_exists(directory.join(name))?;
    }
    Ok(())
}

fn set_global_tracing_subscriber(
    console_log_level: LevelFilter,
    file_log_level: LevelFilter,
    log_writer: impl for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
    json_log_writer: impl for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
) -> eyre::Result<()> {
    // Use custom timer formatting so that we only include minimal info in stdout.
    // The log files contain more accurate time stamps
    let stdout_timer = |writer: &mut Writer| -> std::fmt::Result {
        // TODO: I'm concerned this might be slow if it's parser every time.
        // I think the time crate might have some options for building compile-time
        // parsers
        let time = Local::now().format("%H:%M:%S.%3f");
        write!(writer, "{time}")
    };

    let stdout_layer = fmt::Layer::default()
        .compact()
        .with_timer(stdout_timer as fn(&mut Writer) -> std::fmt::Result)
        .with_filter(console_log_level);

    let log_file_layer = fmt::Layer::default()
        .with_writer(log_writer)
        .with_filter(file_log_level);

    let json_log_file_layer = fmt::Layer::default()
        .json()
        .with_thread_ids(true)
        .with_span_events(FmtSpan::ACTIVE)
        .with_writer(json_log_writer)
        .with_filter(file_log_level);

    let subscriber = Registry::default()
        .with(stdout_layer)
        .with(log_file_layer)
        .with(json_log_file_layer);
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn remove_file_if_exists(path: impl AsRef<Path>) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(err),
        },
    }
}

pub struct TracingGuard {
    log_file_writer: Option<Arc<MutexWriter<MultiWriter<File>>>>,
    gz_log_file_writer: Option<Arc<MutexWriter<GzipLogWriter<MultiWriter<File>>>>>,
    json_log_file_writer: Option<Arc<MutexWriter<MultiWriter<File>>>>,
    gz_json_log_file_writer: Option<Arc<MutexWriter<GzipLogWriter<MultiWriter<File>>>>>,
}

impl TracingGuard {
    fn new() -> Self {
        Self {
            log_file_writer: None,
            gz_log_file_writer: None,
            json_log_file_writer: None,
            gz_json_log_file_writer: None,
        }
    }

    // Called from Drop impl and/or signal handler
    fn finalize(&mut self) {
        // TODO: Should we write to stdout if any of these things fail, particularly
        // finishing the gzip encoders?
        if let Some(log_file_writer) = &mut self.log_file_writer {
            if let Ok(mut writer) = log_file_writer.0.lock() {
                let _ = writer.flush();
            }
        }
        if let Some(json_log_file_writer) = &mut self.json_log_file_writer {
            if let Ok(mut writer) = json_log_file_writer.0.lock() {
                let _ = writer.flush();
            }
        }
        if let Some(gz_log_file_writer) = &mut self.gz_log_file_writer {
            if let Ok(mut writer) = gz_log_file_writer.0.lock() {
                let _ = writer.finish();
            }
        }
        if let Some(gz_json_file_writer) = &mut self.gz_json_log_file_writer {
            if let Ok(mut writer) = gz_json_file_writer.0.lock() {
                let _ = writer.finish();
            }
        }
    }

    fn clone_private(&self) -> Self {
        Self {
            log_file_writer: self.log_file_writer.clone(),
            gz_log_file_writer: self.gz_log_file_writer.clone(),
            json_log_file_writer: self.json_log_file_writer.clone(),
            gz_json_log_file_writer: self.gz_json_log_file_writer.clone(),
        }
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        self.finalize();
    }
}

struct GzipLogWriter<W: Write> {
    encoder: Option<GzEncoder<W>>,
}

impl<W: Write> GzipLogWriter<W> {
    fn finish(&mut self) -> std::io::Result<()> {
        // By taking the encoder, we ensure that finish can never be called twice
        if let Some(encoder) = self.encoder.take() {
            encoder.finish()?;
        }
        Ok(())
    }
}

impl<W: Write> GzipLogWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            encoder: Some(GzEncoder::new(writer, Compression::default())),
        }
    }
}

impl<W: Write> Write for GzipLogWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(encoder) = &mut self.encoder {
            encoder.write(buf)
        } else {
            // We have no encoder, so just silently swallow the bytes?
            // TODO: Or maybe log something to stderr because this is probably something
            // a user should fix ASAP
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(encoder) = &mut self.encoder {
            encoder.flush()
        } else {
            Ok(())
        }
    }
}

struct MutexWriter<W>(Mutex<W>);

impl<W> MutexWriter<W> {
    pub fn new(writer: W) -> Self {
        Self(Mutex::new(writer))
    }
}

impl<W: Write> Write for MutexWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        <&Self as Write>::write(&mut &*self, buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        <&Self as Write>::flush(&mut &*self)
    }
}

impl<'a, W: Write> Write for &'a MutexWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut writer = self
            .0
            .lock()
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to lock mutex for writing"))?;
        writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut writer = self
            .0
            .lock()
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to lock mutex for flushing"))?;
        writer.flush()
    }
}

/// A writer that forwards the data to multiple writers.
struct MultiWriter<W> {
    writers: Vec<W>,
}

impl<W> MultiWriter<W> {
    pub fn from_writers(writers: Vec<W>) -> Self {
        Self { writers }
    }
}

impl<W: Write> Write for MultiWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written_bytes = buf.len();
        for writer in &mut self.writers {
            written_bytes = min(writer.write(buf)?, written_bytes);
        }
        Ok(written_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for writer in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }
}
