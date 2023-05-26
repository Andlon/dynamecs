use std::path::PathBuf;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::timing2::extract_step_timings;

#[test]
fn extract_timings() -> eyre::Result<()> {
    // TODO: Modify iterate_records so that we don't have to collect first
    // (probably remove the internal Result<Record> layer
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/unit_tests/example_logs/basic_app_4_steps.jsonlog");
    let records: Vec<_> = iterate_records(path)?
        .collect::<eyre::Result<Vec<_>>>()?;
    let timings = extract_step_timings(&records).unwrap();

    let trees: Vec<_> = timings.steps().iter().map(|step| step.create_timing_tree()).collect();

    dbg!(trees);

    Ok(())
}