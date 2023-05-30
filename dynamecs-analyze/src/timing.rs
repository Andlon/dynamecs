use std::cmp::max;
use std::collections::{HashMap};
use std::time::Duration;
use eyre::eyre;
use time::OffsetDateTime;
use RecordKind::{SpanEnter, SpanExit};
use crate::{Record, RecordKind, SpanPath, SpanTree, SpanTreeNode};
use std::fmt::Write;
use tabwriter::TabWriter;
use std::io::Write as IoWrite;

pub type TimingTree = SpanTree<DerivedStats>;
type TimingTreeNode<'a> = SpanTreeNode<'a, DerivedStats>;

#[derive(Debug, Clone)]
pub struct DerivedStats {
    pub duration: Duration,
    pub duration_relative_to_parent: Option<f64>,
    pub duration_relative_to_root: Option<f64>,
}

fn update_column_widths_for_line(column_widths: &mut Vec<usize>, line: &str) {
    let mut column_iter = line.split("\t");
    // Update existing column widths
    for (col_width, column_content) in column_widths.iter_mut().zip(column_iter.by_ref()) {
        *col_width = max(*col_width, column_content.len());
    }
    // Push new column widths
    for column_content in column_iter {
        column_widths.push(column_content.len());
    }
}

fn write_table_line(output: &mut String, column_widths: &[usize], line: &str) {
    let padding = 2;
    debug_assert_eq!(line.lines().count(), 1, "line string must consist of a single line");
    for (cell, width) in line.split("\t").zip(column_widths) {
        write!(output, "{cell:width$}", width=width + padding).unwrap();
    }
    writeln!(output).unwrap();
}

fn format_table(header: &str, table: &str) -> String {
    debug_assert_eq!(header.lines().count(), 1, "Header must only have a single line");
    let mut column_widths = vec![];
    update_column_widths_for_line(&mut column_widths, header);
    for line in table.lines() {
        update_column_widths_for_line(&mut column_widths, line);
    }

    let mut output = String::new();
    write_table_line(&mut output, &column_widths, header);
    let header_len = output.len();
    output.push_str(&"═".repeat(header_len));
    writeln!(output).unwrap();

    for line in table.lines() {
        write_table_line(&mut output, &column_widths, line);
    }

    output.push_str(&"═".repeat(header_len));
    writeln!(output).unwrap();

    output
}

pub fn format_timing_tree(tree: &TimingTree) -> String {
    let mut table = String::new();
    write_timing_tree_node(&mut table, tree.root(), &mut vec![]);
    format_table("Duration\tRel parent\tSpan",
                 &table)
}

fn write_timing_tree_node(
    output: &mut String,
    node: TimingTreeNode,
    active_stack: &mut Vec<bool>
) {
    let duration = node.payload().duration;
    write_duration(output, &duration);

    if let Some(proportion) = node.payload().duration_relative_to_parent {
        let percentage = 100.0 * proportion;
        let _ = write!(output, "\t{percentage:5.1} %");
    } else {
        let _ = write!(output, "\t    N/A");
    }
    write!(output, "\t").unwrap();

    if let Some((&parent_is_active, predecessors)) = active_stack.split_last() {
        for &is_active in predecessors {
            if is_active {
                output.push_str("│   ");
            } else {
                output.push_str("    ");
            }
        }
        if parent_is_active {
            output.push_str("├── ");
        } else {
            output.push_str("└── ");
        }
    }

    writeln!(output, "{}", node.path().span_name().unwrap_or("no name? Fix, TODO")).unwrap();
    let num_children = node.count_children();
    for (child_idx, child) in node.visit_children().enumerate() {
        // We say that an ancestor is "active" if it's not yet processing its last child.
        // This criterion lets us avoid drawing excessive numbers of vertical lines,
        // which make for a visually confusing picture.
        let is_last_child = child_idx + 1 == num_children;
        active_stack.push(!is_last_child);
        write_timing_tree_node(output, child, &mut *active_stack);
        active_stack.pop();
    }
}

// TODO: Unit tests for this one?
fn write_duration(output: &mut String, duration: &Duration) {
    let secs = duration.as_secs_f64();
    if 1e-9 <= secs && secs < 1e-6 {
        write!(output, "{:5.1} ns", secs / 1e-9).unwrap();
    } else if 1e-6 <= secs && secs < 1e-3 {
        write!(output, "{:5.1} μs", secs / 1e-6).unwrap();
    } else if 1e-3 <= secs && secs < 1.0 {
        write!(output, "{:5.1} ms", secs / 1e-3).unwrap();
    } else if 1.0 <= secs && secs < 1e3 {
        write!(output, "{:5.1} s ", secs).unwrap();
    } else {
        write!(output, "{:5.1e} s ", secs).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct AccumulatedTimings {
    span_durations: HashMap<SpanPath, Duration>,
}

#[derive(Debug, Clone)]
pub struct AccumulatedStepTimings {
    pub timings: AccumulatedTimings,
    pub step_index: u64,
}

impl AccumulatedTimings {
    pub fn new() -> Self {
        Self { span_durations: Default::default() }
    }

    pub fn merge_with_others<'a>(&mut self, others: impl Iterator<Item=&'a AccumulatedTimings>) {
        for other in others {
            for (path, duration) in &other.span_durations {
                let current_duration = self.span_durations.entry(path.clone())
                    .or_default();
                *current_duration += *duration;
            }
        }
    }
}

impl AccumulatedTimings {
    pub fn create_timing_tree(&self) -> TimingTree {
        let (paths, durations) = self.span_durations
            .iter()
            .map(|(path, duration)| (path.clone(), duration.clone()))
            .unzip();
        SpanTree::from_paths_and_payloads(paths, durations)
            .transform_payloads(|node| {
                let duration = node.payload();
                DerivedStats {
                    duration: *duration,
                    duration_relative_to_parent: node.parent()
                        .map(|parent_node| {
                            let parent_duration = parent_node.payload();
                            let proportion = duration.as_secs_f64() / parent_duration.as_secs_f64();
                            proportion
                        }),
                    duration_relative_to_root: None,
                }
            })
    }
}

#[derive(Debug, Clone)]
pub struct AccumulatedTimingSeries {
    steps: Vec<AccumulatedStepTimings>,
    // TODO: Timing from other sources outside of steps?
}

impl AccumulatedTimingSeries {
    pub fn summarize(&self) -> AccumulatedTimings {
        let mut summary = AccumulatedTimings::new();
        summary.merge_with_others(self.steps().iter().map(|step| &step.timings));
        summary
    }
}

impl AccumulatedTimingSeries {
    pub fn steps(&self) -> &[AccumulatedStepTimings] {
        &self.steps
    }
}

pub fn extract_step_timings<'a>(records: impl IntoIterator<Item=Record>) -> eyre::Result<AccumulatedTimingSeries> {
    // TODO: Collect statistics from spans outside run as well
    find_and_visit_dynamecs_run_span(records.into_iter())
}

pub fn extract_timing_summary<'a>(records: impl IntoIterator<Item=Record>) -> eyre::Result<AccumulatedTimings> {
    extract_step_timings(records).map(|series| series.summarize())
}

fn find_and_visit_dynamecs_run_span<'a>(mut records: impl Iterator<Item=Record>) -> eyre::Result<AccumulatedTimingSeries> {
    // First try to find the `run` span in the records
    while let Some(record) = records.next() {
        if let Some(span) = record.span() {
            if span.name() == "run"
                && record.target() == "dynamecs_app"
                && record.kind() == RecordKind::SpanEnter {
                return visit_dynamecs_run_span(&record, records);
            }
        }
    }

    Err(eyre!("Could not find new event for `run` span of dynamecs among records"))
}

fn visit_dynamecs_run_span<'a>(run_new_record: &Record, remaining_records: impl Iterator<Item=Record>) -> eyre::Result<AccumulatedTimingSeries> {
    let run_thread = run_new_record.thread_id();
    let run_span_path = run_new_record.span_path();
    let mut iter = remaining_records;

    let mut steps = Vec::new();

    while let Some(record) = iter.next() {
        if record.thread_id() == run_thread {
            if let Some(span) = record.span() {
                match (span.name(), record.target(), record.kind()) {
                    ("step", "dynamecs_app", SpanEnter) => {
                        if let Some(step) = visit_dynamecs_step_span(&record, &mut iter)? {
                            // Only collect complete time steps
                            steps.push(step);
                        }
                    },
                    ("run", "dynamecs_app", SpanExit) if record.span_path() == run_span_path => {
                        break;
                    },
                    // TODO: Still accumulate timings for other things?
                    _ => {}
                }
            }
        }
    }

    Ok(AccumulatedTimingSeries {
        steps
    })
}

/// Returns accumulated timings for the next *complete* step in the records.
fn visit_dynamecs_step_span<'a>(
    step_new_record: &Record,
    remaining_records: &mut impl Iterator<Item=Record>
) -> eyre::Result<Option<AccumulatedStepTimings>> {
    let step_path = step_new_record.span_path();

    let mut accumulator = TimingAccumulator::new();
    accumulator.enter_span(step_path.clone(), step_new_record.timestamp().clone())?;

    let step_index = step_new_record.span()
        .and_then(|span| span.fields().pointer("/step_index"))
        .and_then(|value| value.as_u64())
        .ok_or_else(|| eyre!("step span does not have step_index field"))?;

    while let Some(record) = remaining_records.next() {
        if record.thread_id() == step_new_record.thread_id() {
            if let Some(span) = record.span() {
                match record.kind() {
                    SpanEnter => {
                        accumulator.enter_span(record.span_path(),
                                               record.timestamp().clone())?;
                    },
                    SpanExit => {
                        // TODO: use a stack to verify that open/close events are consistent?
                        let mut span_path = record.span_path();
                        // Close events don't report the current span anymore,
                        // so we need to add this to get a path consistent with the
                        // "new" event
                        span_path.push_span_name(span.name().to_string());
                        let is_step_span_path = span_path == step_path;
                        accumulator.exit_span(span_path,
                                              record.timestamp().clone())?;
                        if span.name() == "step"
                            && record.target() == "dynamecs_app"
                            && is_step_span_path {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if accumulator.has_active_spans() {
        // If there are active spans, then the step is not yet complete,
        // so we do not want to include it in accumulation
        // (would lead to inconsistent time between parent and children)
        Ok(None)
    } else {
        Ok(Some(AccumulatedStepTimings {
            timings: AccumulatedTimings { span_durations: accumulator.collect_completed_timings() },
            step_index
        }))
    }
}

#[derive(Debug)]
struct TimingAccumulator {
    completed_durations: HashMap<SpanPath, Duration>,
    enter_timestamps: HashMap<SpanPath, OffsetDateTime>,
}

impl TimingAccumulator {
    pub fn new() -> Self {
        Self { completed_durations: Default::default(), enter_timestamps: Default::default() }
    }

    pub fn enter_span(&mut self, path: SpanPath, timestamp: OffsetDateTime) -> eyre::Result<()> {
        if self.enter_timestamps.insert(path, timestamp).is_some() {
            return Err(eyre!("tried to create new span that is already active (not closed)"));
        }
        Ok(())
    }

    pub fn exit_span(&mut self, path: SpanPath, timestamp_close: OffsetDateTime) -> eyre::Result<()> {
        let timestamp_enter = self.enter_timestamps.remove(&path)
            .ok_or_else(|| eyre!("found close event for span that is not currently active. Span path: {path}"))?;
        let span_duration: Duration = (timestamp_close - timestamp_enter).unsigned_abs();
        let accumulated_duration = self.completed_durations.entry(path)
            .or_default();
        *accumulated_duration += span_duration;
        Ok(())
    }

    pub fn has_active_spans(&self) -> bool {
        !self.enter_timestamps.is_empty()
    }

    pub fn collect_completed_timings(self) -> HashMap<SpanPath, Duration> {
        self.completed_durations
    }
}

