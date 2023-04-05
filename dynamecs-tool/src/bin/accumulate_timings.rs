use std::error::Error;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::timing::accumulate_timings;

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(arg) = std::env::args().skip(1).next() {
        let records: Vec<_> = iterate_records(&arg)?.collect::<Result<_, _>>()?;
        let timings = accumulate_timings(records)?;
        dbg!(&timings);
    } else {
        eprintln!("No path to log file provided, exiting...");
    }
    Ok(())
}
