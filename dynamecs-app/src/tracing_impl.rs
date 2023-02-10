use std::fs::{create_dir_all, File};
use std::io::BufWriter;
use std::sync::Mutex;
use chrono::{Local};
use eyre::WrapErr;
use tracing_subscriber::{fmt, Registry};
use tracing_subscriber::fmt::format::{Writer};
use tracing_subscriber::prelude::*;
use crate::get_output_path;

/// Sets up `tracing`.
///
/// TODO: Describe what it actually does, i.e. log to stdout, file etc.
pub fn setup_tracing() -> eyre::Result<()> {
    let log_dir = get_output_path().join("logs");
    let log_file_path = log_dir.join("dynamecs_app.log");
    let json_log_file_path = log_dir.join("dynamecs_app.json");

    // Use ISO 8601 / RFC 3339, but replace colons with dots, since colons are
    // not valid in Windows filenames (and awkward on Unix)
    let timestamp = format!("{}", Local::now().format("%+")).replace(":", ".");
    let archive_dir = log_dir.join("archive");
    let archive_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.log"));
    let archive_json_log_file_path = archive_dir.join(format!("dynamecs_app.{timestamp}.json"));
    create_dir_all(&log_dir)
        .wrap_err("failed to create log directory")?;
    create_dir_all(&archive_dir)
        .wrap_err("failed to create log archive directory")?;

    let log_file = BufWriter::new(File::create(log_file_path)
        .wrap_err("failed to create main log file")?);
    let json_log_file = BufWriter::new(File::create(json_log_file_path)
        .wrap_err("failed to create json log file")?);
    // TODO: Could maybe combine both main and archive files into a single Layer
    // for possible performance benefits, instead of processing them separately
    let archive_log_file = BufWriter::new(File::create(archive_log_file_path)
        .wrap_err("failed to create archive log file")?);
    let archive_json_log_file = BufWriter::new(File::create(archive_json_log_file_path)
        .wrap_err("failed to create archive json log file")?);

    // Use custom timer formatting so that we only include minimal info in stdout.
    // The log files contain more accurate time stamps
    let stdout_timer = |writer: &mut Writer| -> std::fmt::Result {
        let time = Local::now().format("%H:%M:%S.%3f");
        write!(writer, "{time}")
    };

    let stdout_layer = fmt::Layer::default()
        .compact()
        .with_timer(stdout_timer as fn(&mut Writer) -> std::fmt::Result);
    let log_file_layer = fmt::Layer::default()
        .with_writer(Mutex::new(log_file));
    let json_log_file_layer = fmt::Layer::default()
        .json()
        .with_writer(Mutex::new(json_log_file));
    let archive_log_file_layer = fmt::Layer::default()
        .with_writer(Mutex::new(archive_log_file));
    let archive_json_log_file_layer = fmt::Layer::default()
        .json()
        .with_writer(Mutex::new(archive_json_log_file));

    let subscriber = Registry::default()
        .with(stdout_layer)
        .with(log_file_layer)
        .with(json_log_file_layer)
        .with(archive_log_file_layer)
        .with(archive_json_log_file_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}