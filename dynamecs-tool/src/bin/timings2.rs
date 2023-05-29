use std::error::Error;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::{SpanTree, SpanTreeNode};
use dynamecs_analyze::timing2::{extract_step_timings, format_timing_tree};

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(arg) = std::env::args().skip(1).next() {
        let records: Vec<_> = iterate_records(arg)?
            .collect::<Result<Vec<_>, _>>()?;
        let timings = extract_step_timings(&records)?;

        let trees: Vec<_> = timings.steps().iter().map(|step| step.create_timing_tree()).collect();

        for tree in trees {
            // TODO: Need step index since they're not necessarily consecutive
            // (due to checkpointing, manual manipulation of step indices
            //  or somehow incomplete logs etc.)
            println!("{}", format_timing_tree(&tree));
        }

        Ok(())
    } else {
        Err(Box::from("missing path to log file"))
    }
}