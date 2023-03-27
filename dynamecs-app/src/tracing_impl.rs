use crate::cli::CliOptions;
use crate::get_output_dir;
use chrono::Local;
use clap::Parser;
use eyre::WrapErr;
use std::cmp::min;
use std::fs::{create_dir_all, File};
use std::io::{ErrorKind, Write};
use std::io::Error as IoError;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use flate2::Compression;
use flate2::write::GzEncoder;
use tracing::info;
use tracing_subscriber::fmt::format::{FmtSpan, Writer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, Registry};

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
    let log_dir = get_output_dir().join("logs");
    let log_file_path = log_dir.join("dynamecs_app.log");
    let json_log_file_path = log_dir.join("dynamecs_app.json.gz");

    // Use ISO 8601 / RFC 3339, but replace colons with dots, since colons are
    // not valid in Windows filenames (and awkward on Unix)
    let timestamp = format!("{}", Local::now().format("%+")).replace(":", ".");
    let archive_dir = log_dir.join("archive");
    let archive_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.log"));
    let archive_json_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.json.gz"));
    create_dir_all(&log_dir).wrap_err("failed to create log directory")?;
    create_dir_all(&archive_dir).wrap_err("failed to create log archive directory")?;

    let log_file = File::create(&log_file_path).wrap_err("failed to create main log file")?;
    let json_log_file = File::create(&json_log_file_path).wrap_err("failed to create json log file")?;
    let archive_log_file = File::create(&archive_log_file_path).wrap_err("failed to create archive log file")?;
    let archive_json_log_file =
        File::create(&archive_json_log_file_path).wrap_err("failed to create archive json log file")?;

    // Use custom timer formatting so that we only include minimal info in stdout.
    // The log files contain more accurate time stamps
    let stdout_timer = |writer: &mut Writer| -> std::fmt::Result {
        let time = Local::now().format("%H:%M:%S.%3f");
        write!(writer, "{time}")
    };

    let cli_options = CliOptions::parse();

    let stdout_layer = fmt::Layer::default()
        .compact()
        .with_timer(stdout_timer as fn(&mut Writer) -> std::fmt::Result)
        .with_filter(cli_options.console_log_level);

    let log_file_writer = Mutex::new(MultiWriter::from_writers(vec![log_file, archive_log_file]));
    let log_file_layer = fmt::Layer::default()
        .with_writer(log_file_writer)
        .with_filter(cli_options.file_log_level);

    let json_writers = MultiWriter::from_writers(vec![
        json_log_file,
        archive_json_log_file,
    ]);
    let json_gzip_encoder = GzEncoder::new(json_writers, Compression::default());
    let json_log_file_writer = Arc::new(MutexWriter::new(json_gzip_encoder));
    let json_log_file_layer = fmt::Layer::default()
        .json()
        .with_span_events(FmtSpan::ACTIVE)
        .with_writer(json_log_file_writer.clone())
        .with_filter(cli_options.file_log_level);

    let subscriber = Registry::default()
        .with(stdout_layer)
        .with(log_file_layer)
        .with(json_log_file_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    let working_dir = std::env::current_dir().wrap_err("failed to retrieve current working directory")?;
    info!(target: "dynamecs_app", "Working directory: {}", working_dir.display());
    info!(target: "dynamecs_app", "Logging text to stdout with log level {}", cli_options.console_log_level.to_string());
    info!(target: "dymamecs_app", "Logging text to file {} with log level {}", log_file_path.display(), cli_options.file_log_level);
    info!(target: "dynamecs_app", "Logging JSON to file {} with log level {}", json_log_file_path.display(), cli_options.file_log_level);
    info!(target: "dynamecs_app", "Archived log file path:  {}", archive_log_file_path.display());
    info!(target: "dynamecs_app", "Archived JSON log file path: {}", archive_json_log_file_path.display());

    Ok(TracingGuard { json_log_file_writer })
}

pub struct TracingGuard {
    json_log_file_writer: Arc<MutexWriter<GzEncoder<MultiWriter<File>>>>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        let writer_ref = &mut self.json_log_file_writer.deref();
        let _result = writer_ref.flush();
        if let Ok(gzip_encoder) = self.json_log_file_writer.0.lock() {
            let _result = gzip_encoder.finish();
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
        let mut writer = self.0.lock()
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to lock mutex for writing"))?;
        writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut writer = self.0.lock()
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
