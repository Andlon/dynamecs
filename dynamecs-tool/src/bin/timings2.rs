use std::error::Error;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::{SpanTree, SpanTreeNode};
use dynamecs_analyze::timing2::{extract_step_timings, format_timing_tree};
use std::fmt::Write;

fn add_prefix_to_multiline_string(string: &str, prefix: &str) -> String {
    let mut output = String::new();
    for line in string.lines() {
        let _ = writeln!(output, "{prefix}{line}");
    }
    output
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(arg) = std::env::args().skip(1).next() {
        let mut records_result_iter = iterate_records(arg)?;
        let records_iter = records_result_iter
            // TODO: Use peeking_take_while or something so that we can
            // check for errors in the remaining records in combination with .by_ref()
            .map_while(|record| record.ok());

        let timings = extract_step_timings(records_iter)?;
        for step in timings.steps() {
            let tree = step.timings.create_timing_tree();
            println!("Timings for step index {}", step.step_index);
            println!("════════════════════════════════");

            let prefixed_tree = add_prefix_to_multiline_string(&format_timing_tree(&tree), "  ");
            println!("{prefixed_tree}");
            println!();
        }

        let summary_tree = timings.summarize().create_timing_tree();
        println!("Aggregate timings");
        println!("════════════════════════════════");
        let prefixed_summary_tree = add_prefix_to_multiline_string(
            &format_timing_tree(&summary_tree), "  ");
        println!("{prefixed_summary_tree}");
        println!();

        Ok(())
    } else {
        Err(Box::from("missing path to log file"))
    }
}