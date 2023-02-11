use std::path::PathBuf;
use structopt::StructOpt;
use tracing::Level;
use crate::get_output_path;

#[derive(StructOpt)]
pub struct CliOptions {
    #[structopt(
        short,
        long,
        help = "The path (relative or absolute) to a scenario-specific JSON5 configuration file."
    )]
    pub config_file: Option<PathBuf>,
    #[structopt(
        long,
        help = "A scenario configuration as a JSON5 string."
    )]
    pub config_string: Option<String>,
    #[structopt(
        short = "o",
        long = "output-dir",
        help = "Output base directory, relative or absolute.",
        default_value = get_output_path().to_str().expect("will always be valid string")
    )]
    pub output_dir: PathBuf,
    #[structopt(long = "dt", help = "Override the time step used for the simulation.")]
    pub dt: Option<f64>,
    #[structopt(
        long = "max-steps",
        help = "Maximum number of simulation steps to take (by default infinite)"
    )]
    pub max_steps: Option<usize>,
    #[structopt(
        long = "write-checkpoints",
        help = "Write a checkpoint file to disk after every timestep"
    )]
    pub write_checkpoints: bool,
    #[structopt(
        long = "restore-checkpoint",
        help = "Restore the simulation state from a checkpoint file and continue the simulation"
    )]
    pub restore_checkpoint: Option<PathBuf>,
    #[structopt(
        long,
        default_value = "info",
        help = "Log level to use for logging to the console. \
                Possible values: error, warn, info, debug, trace.",
    )]
    pub console_log_level: Level,
    #[structopt(
        long,
        default_value = "debug",
        help = "Log level to use for text and JSON log files. \
                Possible values: error, warn, info, debug, trace.",
    )]
    pub file_log_level: Level,
}
