use std::error::Error;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::timing::{extract_step_timings, format_timing_tree};
use std::fmt::Write;
use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Timing {
        #[arg(short, long)]
        logfile: PathBuf,
        /// Only aggregate timings across all steps in the log file will be returned.
        #[arg(short, long)]
        aggregate: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Timing { logfile, aggregate } => {
            let records_result_iter = iterate_records(logfile)?;
            let records_iter = records_result_iter
                // TODO: Use peeking_take_while or something so that we can
                // check for errors in the remaining records in combination with .by_ref()
                .map_while(|record| record.ok());

            let timings = extract_step_timings(records_iter)?;
            if !aggregate {
                for step in timings.steps() {
                    let tree = step.timings.create_timing_tree();
                    println!("Timings for step index {}", step.step_index);
                    println!("════════════════════════════════");

                    let prefixed_tree = add_prefix_to_multiline_string(&format_timing_tree(&tree), "  ");
                    println!("{prefixed_tree}");
                    println!();
                }
            }

            let summary_tree = timings.summarize().create_timing_tree();
            println!("Aggregate timings");
            println!("════════════════════════════════");
            println!();
            let prefixed_summary_tree = add_prefix_to_multiline_string(
                &format_timing_tree(&summary_tree), "  ");
            println!("{prefixed_summary_tree}");
            println!();
            println!("Number of completed time steps: {}", timings.steps().len());
        }
    }

    Ok(())
}

fn add_prefix_to_multiline_string(string: &str, prefix: &str) -> String {
    let mut output = String::new();
    for line in string.lines() {
        let _ = writeln!(output, "{prefix}{line}");
    }
    output
}