use std::collections::{BTreeMap, HashMap};
use std::iter::Peekable;
use std::time::Duration;
use eyre::eyre;
use time::OffsetDateTime;
use crate::{Record, RecordKind, Span, SpanPath};
use crate::RecordKind::{SpanClose, SpanNew};

// pub struct TimingTreeNode {
//     span: Span,
//     duration: Duration,
//     children: Vec<TimingTreeNode>,
// }
//
// pub struct StepTimings {
//     steps_trees: Vec<BTreeMap<String, TimingTreeNode>>,
//     step_index: usize,
// }
//
// pub struct StepTimingsCollection {
//     per_thread_timings: BTreeMap<String, TimingTreeNode>,
//     steps: Vec<TimingTreeNode>,
// }

// struct StepTimingAccumulator {
//     // a stack of span names coupled with timestamp of its *new* event
//     stack: Vec<(String, OffsetDateTime)>,
// }
//
// impl StepTimingAccumulator {
//     pub fn new() -> Self {
//         Self { stack: Vec::new() }
//     }
//
//     pub fn new_span(&mut self, span: &Span, new_timestamp: &OffsetDateTime) {
//         self.stack.push((span.name.clone(), *new_timestamp));
//     }
//
//     pub fn close_span(&mut self, span: &Span, close_timestamp: &OffsetDateTime) -> eyre::Result<()> {
//         // TODO: Proper errors
//         let (popped_span, new_timestamp) = self.stack.pop().ok_or_else(|| eyre!("tried to close span that was not opened"))?;
//         if span.name() != popped_span {
//             return Err(eyre!("unexpected span closed"));
//         }
//         let duration = *close_timestamp - new_timestamp;
//
//         Ok(())
//     }
// }

// pub struct StepTimings {
//     steps: Vec<()>,
//
// }

pub struct AccumulatedTimings {
    span_durations: HashMap<SpanPath, Duration>,
}

pub struct AccumulatedTimingSeries {
    steps: Vec<AccumulatedTimings>,
    // TODO: Timing from other sources outside of steps?
}

pub fn extract_step_timings<'a>(records: impl Iterator<Item=&'a Record>) -> eyre::Result<AccumulatedTimingSeries> {
    // TODO: Collect statistics from spans outside run as well
    find_and_visit_dynamecs_run_span(records)
}

fn find_and_visit_dynamecs_run_span<'a>(mut records: impl Iterator<Item=&'a Record>) -> eyre::Result<AccumulatedTimingSeries> {
    // First try to find the `run` span in the records
    while let Some(record) = records.next() {
        if let Some(span) = record.span() {
            if span.name() == "run"
                && record.target() == "dynamecs_app"
                && record.kind() == RecordKind::SpanNew {
                return visit_dynamecs_run_span(record, records);
            }
        }
    }

    Err(eyre!("Could not find new event for `run` span of dynamecs among records"))
}

fn visit_dynamecs_run_span<'a>(run_new_record: &Record, remaining_records: impl Iterator<Item=&'a Record>) -> eyre::Result<AccumulatedTimingSeries> {
    let run_thread = run_new_record.thread_id();
    let mut iter = remaining_records.peekable();

    while let Some(peek_record) = iter.peek() {
        if peek_record.thread_id() == run_thread {
            if let Some(span) = peek_record.span() {
                match (span.name(), peek_record.target(), peek_record.kind()) {
                    ("step", "dynamecs_app", SpanNew) => {
                        visit_dynamecs_step_span(peek_record, &mut iter)?;
                    },
                    ("run", "dynamecs_app", SpanClose) => {

                    },
                    // TODO: Still accumulate timings for other things?
                    _ => {}
                }
            }
        }
    }

    todo!()
}

/// Returns accumulated timings for the next *complete* step in the records.
fn visit_dynamecs_step_span<'a>(
    step_new_record: &Record,
    remaining_records: &mut impl Iterator<Item=&'a Record>
) -> eyre::Result<Option<AccumulatedTimings>> {
    let step_path = step_new_record.span_path();

    let mut accumulator = TimingAccumulator::new();
    accumulator.new_span(step_path.clone(), step_new_record.timestamp().clone())?;

    while let Some(peek_record) = remaining_records.next() {
        if peek_record.thread_id() == step_new_record.thread_id() {
            if let Some(span) = peek_record.span() {
                match peek_record.kind() {
                    SpanNew => accumulator.new_span(peek_record.span_path(),
                                                    peek_record.timestamp().clone())?,
                    SpanClose => {
                        let span_path = peek_record.span_path();
                        let is_step_span_path = span_path == step_path;
                        accumulator.close_span(span_path,
                                               peek_record.timestamp().clone())?;
                        if span.name() == "step"
                            && peek_record.target() == "dynamecs_app"
                            && is_step_span_path {
                            break;
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    todo!("get timings from accumulator");

    // If we reached the end of the iterator without running into a "close" event,
    // then we discard the step data we collected

    Ok(None)
}

#[derive(Debug)]
struct TimingAccumulator {
    completed_durations: HashMap<SpanPath, Duration>,
    timestamps_open: HashMap<SpanPath, OffsetDateTime>,
}

impl TimingAccumulator {
    pub fn new() -> Self {
        Self { completed_durations: Default::default(), timestamps_open: Default::default() }
    }

    pub fn new_span(&mut self, path: SpanPath, timestamp: OffsetDateTime) -> eyre::Result<()> {
        if self.timestamps_open.insert(path, timestamp).is_some() {
            return Err(eyre!("tried to create new span that is already active (not closed)"));
        }
        Ok(())
    }

    pub fn close_span(&mut self, path: SpanPath, timestamp_close: OffsetDateTime) -> eyre::Result<()> {
        let timestamp_new = self.timestamps_open.remove(&path)
            .ok_or_else(|| eyre!("found close event for span that is not currently active. Span path: {path}"))?;
        let span_duration: Duration = (timestamp_close - timestamp_new).unsigned_abs();
        let mut accumulated_duration = self.completed_durations.entry(path)
            .or_default();
        *accumulated_duration += span_duration;
        Ok(())
    }
}

