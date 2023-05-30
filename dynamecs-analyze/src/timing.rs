use std::cmp::max;
use std::collections::{HashMap};
use std::time::Duration;
use eyre::eyre;
use time::OffsetDateTime;
use RecordKind::{SpanEnter, SpanExit};
use crate::{Record, RecordKind, SpanPath, SpanTree, SpanTreeNode};
use std::fmt::Write;
use std::iter;

pub type TimingTree = SpanTree<DerivedStats>;
type TimingTreeNode<'a> = SpanTreeNode<'a, DerivedStats>;

/// Statistics measured directly from logs.
#[derive(Debug, Clone, Default)]
pub struct DirectStats {
    /// Total accumulated duration for the span.
    pub duration: Duration,
    /// Number of times the span was entered and subsequently *exited*.
    pub count: u64,
}

impl DirectStats {
    pub fn from_single_duration(duration: Duration) -> Self {
        Self { duration, count: 1 }
    }

    pub fn combine_mut(&mut self, other: &DirectStats) {
        self.duration += other.duration;
        self.count += other.count;
    }
}

#[derive(Debug, Clone)]
pub struct DerivedStats {
    pub duration: Duration,
    pub count: u64,
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

fn write_table_line(
    output: &mut String,
    line: &str,
    column_widths: &[usize],
    alignments: &[Alignment],
) {
    let padding = 2;
    debug_assert_eq!(line.lines().count(), 1, "line string must consist of a single line");
    let alignment_iter = alignments.iter().chain(iter::repeat(&Alignment::Left));
    for ((cell, width), alignment) in line.split("\t").zip(column_widths).zip(alignment_iter) {
        match alignment {
            Alignment::Left => write!(output, "{cell:width$}", width=width).unwrap(),
            Alignment::Right => write!(output, "{cell: >width$}", width=width).unwrap()
        }
        for _ in 0 .. padding {
            output.push(' ');
        }
    }
    writeln!(output).unwrap();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Alignment {
    Left,
    Right
}

fn format_table(header: &str, table: &str, alignments: &[Alignment]) -> String {
    debug_assert_eq!(header.lines().count(), 1, "Header must only have a single line");
    let mut column_widths = vec![];
    update_column_widths_for_line(&mut column_widths, header);
    for line in table.lines() {
        update_column_widths_for_line(&mut column_widths, line);
    }

    let mut output = String::new();
    // Use default alignment for table headers, apply alignments only to cells
    write_table_line(&mut output, header, &column_widths, &[]);
    let header_len = output.len();
    output.push_str(&"═".repeat(header_len));
    writeln!(output).unwrap();

    for line in table.lines() {
        write_table_line(&mut output, line, &column_widths, alignments);
    }

    output.push_str(&"═".repeat(header_len));
    writeln!(output).unwrap();

    output
}

pub fn format_timing_tree(tree: &TimingTree) -> String {
    let mut table = String::new();
    write_timing_tree_node(&mut table, tree.root(), &mut vec![]);
    use Alignment::{Left, Right};
    format_table("Total\tAverage\tCount\tRel parent\tRel root\tSpan",
                 &table,
                 &vec![Right, Right, Right, Right, Right, Left])
}

fn write_proportion(output: &mut String, proportion: Option<f64>) {
    if let Some(proportion) = proportion {
        let percentage = 100.0 * proportion;
        let _ = write!(output, "{percentage:5.1} %");
    } else {
        let _ = write!(output, "    N/A");
    }
}

fn write_timing_tree_node(
    output: &mut String,
    node: TimingTreeNode,
    active_stack: &mut Vec<bool>
) {
    let duration = node.payload().duration;
    let count = node.payload().count;
    write_duration(output, &duration);
    write!(output, "\t").unwrap();
    write_duration(output, &duration.div_f64(count as f64));

    write!(output, "\t{count}").unwrap();

    write!(output, "\t").unwrap();
    write_proportion(output, node.payload().duration_relative_to_parent);
    write!(output, "\t").unwrap();
    write_proportion(output, node.payload().duration_relative_to_root);

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
    span_stats: HashMap<SpanPath, DirectStats>,
}

#[derive(Debug, Clone)]
pub struct AccumulatedStepTimings {
    pub timings: AccumulatedTimings,
    pub step_index: u64,
}

impl AccumulatedTimings {
    pub fn new() -> Self {
        Self { span_stats: Default::default() }
    }

    pub fn merge_with_others<'a>(&mut self, others: impl Iterator<Item=&'a AccumulatedTimings>) {
        for other in others {
            for (path, stats) in &other.span_stats {
                let current_stats = self.span_stats.entry(path.clone())
                    .or_default();
                current_stats.combine_mut(&stats);
            }
        }
    }
}

impl AccumulatedTimings {
    pub fn create_timing_tree(&self) -> TimingTree {
        let (paths, durations) = self.span_stats
            .iter()
            .map(|(path, duration)| (path.clone(), duration.clone()))
            .unzip();
        SpanTree::from_paths_and_payloads(paths, durations)
            .transform_payloads(|node| {
                let stats = node.payload();
                let duration = stats.duration;
                DerivedStats {
                    duration: stats.duration,
                    count: stats.count,
                    duration_relative_to_parent: node.parent()
                        .map(|parent_node| {
                            let parent_duration = parent_node.payload().duration;
                            let proportion = duration.as_secs_f64() / parent_duration.as_secs_f64();
                            proportion
                        }),
                    duration_relative_to_root: Some(node.root())
                        // TODO: There will always be a root at the moment,\
                        // but we'll probably later have root nodes with an optional Duration"
                        .map(|root_node| {
                            let root_duration = root_node.payload().duration;
                            let proportion = duration.as_secs_f64() / root_duration.as_secs_f64();
                            proportion
                        })
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
            timings: AccumulatedTimings { span_stats: accumulator.collect_completed_statistics() },
            step_index
        }))
    }
}

#[derive(Debug)]
struct TimingAccumulator {
    completed_statistics: HashMap<SpanPath, DirectStats>,
    enter_timestamps: HashMap<SpanPath, OffsetDateTime>,
}

impl TimingAccumulator {
    pub fn new() -> Self {
        Self { completed_statistics: Default::default(), enter_timestamps: Default::default() }
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
        let accumulated_stats = self.completed_statistics.entry(path)
            .or_default();
        accumulated_stats.combine_mut(&DirectStats::from_single_duration(span_duration));
        Ok(())
    }

    pub fn has_active_spans(&self) -> bool {
        !self.enter_timestamps.is_empty()
    }

    pub fn collect_completed_statistics(self) -> HashMap<SpanPath, DirectStats> {
        self.completed_statistics
    }
}

