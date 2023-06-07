use std::error::Error;
use serde_json::json;
use time::Duration;
use dynamecs_analyze::{Record, RecordBuilder, Span};
use dynamecs_analyze::timing::{extract_step_timings, format_timing_tree};
use crate::unit_tests::IncrementalTimestamp;

fn synthetic_records1() -> Vec<Record> {
    let mut next_date = IncrementalTimestamp::default();

    let obj = serde_json::Value::Object(Default::default());

    // Define helper functions to create spans
    let run = || Span::from_name_and_fields("run", obj.clone());
    let step = |i: i64| Span::from_name_and_fields("step", json!({ "step_index": i }));
    let simulate = || Span::from_name_and_fields("simulate", obj.clone());
    let assemble = || Span::from_name_and_fields("assemble", obj.clone());
    let solve = || Span::from_name_and_fields("solve", obj.clone());
    let occasional = || Span::from_name_and_fields("occasional", obj.clone());

    vec![
        // Arbitrary event before we enter main spans
        RecordBuilder::event()
            .debug()
            .message("msg1")
            .target("target1")
            .timestamp(next_date.current()),
        // Enter "run" span
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(run())
            .spans(vec![run()])
            .target("dynamecs_app"),
        // Enter timestep 0
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(1)))
            .span(step(0))
            .spans(vec![run(), step(0)])
            .target("dynamecs_app"),
        // Arbitrary event inside span
        RecordBuilder::event()
            .debug()
            .timestamp(next_date.advance_by(Duration::seconds(2)))
            .span(step(0))
            .spans(vec![run(), step(0)])
            .message("msg2")
            .target("target2"),
        // Enter simulate
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(simulate())
            .spans(vec![run(), step(0), simulate()])
            .target("target3"),
        // Enter assemble
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(assemble())
            .spans(vec![run(), step(0), simulate(), assemble()])
            .target("target3"),
        // Exit assemble
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(3)))
            .span(assemble())
            .spans(vec![run(), step(0), simulate()])
            .target("target3"),
        // Enter solve
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(solve())
            .spans(vec![run(), step(0), simulate(), solve()])
            .target("target3"),
        // Exit solve
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(2)))
            .span(solve())
            .spans(vec![run(), step(0), simulate()])
            .target("target3"),
        // Exit simulate
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(1)))
            .span(simulate())
            .spans(vec![run(), step(0)])
            .target("target3"),
        // Exit timestep 0
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(step(0))
            .spans(vec![run()])
            .target("dynamecs_app"),
        // Enter timestep 1
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(step(1))
            .spans(vec![run(), step(1)])
            .target("dynamecs_app"),
        // Arbitrary event inside span
        RecordBuilder::event()
            .debug()
            .timestamp(next_date.advance_by(Duration::seconds(1)))
            .span(step(1))
            .spans(vec![run(), step(1)])
            .message("msg2")
            .target("target2"),
        // Enter simulate
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(2)))
            .span(simulate())
            .spans(vec![run(), step(1), simulate()])
            .target("target3"),
        // Enter assemble
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(assemble())
            .spans(vec![run(), step(1), simulate(), assemble()])
            .target("target3"),
        // Exit assemble
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(2)))
            .span(assemble())
            .spans(vec![run(), step(1), simulate()])
            .target("target3"),
        // Enter assemble
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(assemble())
            .spans(vec![run(), step(1), simulate(), assemble()])
            .target("target3"),
        // Exit assemble
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(3)))
            .span(assemble())
            .spans(vec![run(), step(1), simulate()])
            .target("target3"),
        // Enter solve
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(solve())
            .spans(vec![run(), step(1), simulate(), solve()])
            .target("target3"),
        // Exit solve
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(2)))
            .span(solve())
            .spans(vec![run(), step(1), simulate()])
            .target("target3"),
        // Enter occasional
        RecordBuilder::span_enter()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(occasional())
            .spans(vec![run(), step(1), simulate(), occasional()])
            .target("target3"),
        // Exit occasional
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(4)))
            .span(occasional())
            .spans(vec![run(), step(1), simulate()])
            .target("target3"),
        // Exit simulate
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(1)))
            .span(simulate())
            .spans(vec![run(), step(1)])
            .target("target3"),
        // Exit timestep 1
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(0)))
            .span(step(1))
            .spans(vec![run()])
            .target("dynamecs_app"),
        // Exit run
        RecordBuilder::span_exit()
            .info()
            .timestamp(next_date.advance_by(Duration::seconds(1)))
            .span(run())
            .target("dynamecs_app"),
    ].into_iter()
        .map(|builder| builder.thread_id("ThreadId(0)").build())
        .collect()
}

#[test]
fn test_extract_step_timings_synthetic1() -> Result<(), Box<dyn Error>> {
    let records = synthetic_records1();

    let timings = extract_step_timings(records.into_iter())?;

    assert_eq!(timings.steps().len(), 2);

    let tree0 = timings.steps()[0].timings.create_timing_tree();
    let tree1 = timings.steps()[1].timings.create_timing_tree();
    insta::assert_snapshot!(format_timing_tree(&tree0));
    insta::assert_snapshot!(format_timing_tree(&tree1));

    let summary = timings.summarize().create_timing_tree();
    insta::assert_snapshot!(format_timing_tree(&summary));

    Ok(())
}

#[test]
fn test_extract_step_timings_synthetic1_incomplete() -> Result<(), Box<dyn Error>> {
    // Make the test set incomplete by cutting off records somewhere after
    let records: Vec<_> = synthetic_records1().into_iter().take(19).collect();

    let timings = extract_step_timings(records.into_iter())?;

    assert_eq!(timings.steps().len(), 1);

    let tree0 = timings.steps()[0].timings.create_timing_tree();
    insta::assert_snapshot!(format_timing_tree(&tree0));

    let summary = timings.summarize().create_timing_tree();
    // Since the summary is incomplete, and there's only one step,
    // the summary of a single step is just equivalent to the single step
    assert_eq!(format_timing_tree(&summary), format_timing_tree(&tree0));

    Ok(())
}