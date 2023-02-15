use crate::get_output_path;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;

#[derive(Parser)]
pub struct CliOptions {
    #[arg(
        short,
        long,
        help = "The path (relative or absolute) to a scenario-specific JSON5 configuration file."
    )]
    pub config_file: Option<PathBuf>,
    #[arg(long, help = "A scenario configuration as a JSON5 string.")]
    pub config_string: Option<String>,
    #[arg(
        short = 'o',
        long = "output-dir",
        help = "Output base directory, relative or absolute.",
        default_value = get_output_path().to_str().expect("will always be valid string")
    )]
    pub output_dir: PathBuf,
    #[arg(long = "dt", help = "Override the time step used for the simulation.")]
    pub dt: Option<f64>,
    #[arg(
        long = "max-steps",
        help = "Maximum number of simulation steps to take (by default infinite)"
    )]
    pub max_steps: Option<usize>,
    #[arg(
        long = "write-checkpoints",
        help = "Write a checkpoint file to disk after every timestep"
    )]
    pub write_checkpoints: bool,
    #[arg(
        long = "restore-checkpoint",
        help = "Restore the simulation state from a checkpoint file and continue the simulation"
    )]
    pub restore_checkpoint: Option<PathBuf>,
    #[arg(
        long,
        default_value = "info",
        help = "Log level to use for logging to the console. \
                Possible values: off, error, warn, info, debug, trace."
    )]
    pub console_log_level: LevelFilter,
    #[arg(
        long,
        default_value = "debug",
        help = "Log level to use for text and JSON log files. \
                Possible values: off, error, warn, info, debug, trace."
    )]
    pub file_log_level: LevelFilter,
    #[arg(
        long = "override",
        help = "Override a configuration option using the syntax <path.in.json>=<new value>. \
        Multiple overrides are applied in sequence."
    )]
    pub overrides: Vec<String>,
}
