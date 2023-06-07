use crate::{Record, RecordKind, SpanPath, SpanTree, SpanTreeNode};
use eyre::eyre;
use std::cmp::max;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Write;
use std::iter;
use std::time::Duration;
use time::OffsetDateTime;
use RecordKind::{SpanEnter, SpanExit};

pub type TimingTree = SpanTree<Option<DerivedStats>>;
type TimingTreeNode<'a> = SpanTreeNode<'a, Option<DerivedStats>>;

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

fn write_table_line(output: &mut String, line: &str, column_widths: &[usize], alignments: &[Alignment]) {
    let padding = 2;
    debug_assert_eq!(line.lines().count(), 1, "line string must consist of a single line");
    let alignment_iter = alignments.iter().chain(iter::repeat(&Alignment::Left));
    for ((cell, width), alignment) in line.split("\t").zip(column_widths).zip(alignment_iter) {
        match alignment {
            Alignment::Left => write!(output, "{cell:width$}", width = width).unwrap(),
            Alignment::Right => write!(output, "{cell: >width$}", width = width).unwrap(),
        }
        for _ in 0..padding {
            output.push(' ');
        }
    }
    writeln!(output).unwrap();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Alignment {
    Left,
    Right,
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
    if let Some(root) = tree.root() {
        write_timing_tree_node(&mut table, root, &mut vec![]);
    }
    use Alignment::{Left, Right};
    format_table(
        "Total\tAverage\tCount\tRel parent\tRel root\tSpan",
        &table,
        &vec![Right, Right, Right, Right, Right, Left],
    )
}

fn write_proportion(output: &mut String, proportion: Option<f64>) {
    if let Some(proportion) = proportion {
        let percentage = 100.0 * proportion;
        let _ = write!(output, "{percentage:5.1} %");
    } else {
        let _ = write!(output, "    N/A");
    }
}

fn write_timing_tree_node(output: &mut String, node: TimingTreeNode, active_stack: &mut Vec<bool>) {
    let optional_stats = node.payload().as_ref();
    let duration = optional_stats.map(|stats| stats.duration);
    let count = optional_stats.map(|stats| stats.count);
    write_duration(output, duration);
    write!(output, "\t").unwrap();

    let avg_duration = duration
        .zip(count)
        .map(|(duration, count)| duration.div_f64(count as f64));
    write_duration(output, avg_duration);

    if let Some(count) = count {
        write!(output, "\t{count}").unwrap();
    } else {
        write!(output, "\tN/A").unwrap();
    }

    write!(output, "\t").unwrap();
    let duration_relative_to_parent = optional_stats.and_then(|stats| stats.duration_relative_to_parent);
    write_proportion(output, duration_relative_to_parent);
    write!(output, "\t").unwrap();
    let duration_relative_to_root = optional_stats.and_then(|stats| stats.duration_relative_to_root);
    write_proportion(output, duration_relative_to_root);

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

    writeln!(output, "{}", node.path().span_name().unwrap_or("<root span>")).unwrap();
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
fn write_duration(output: &mut String, duration: Option<Duration>) {
    if let Some(duration) = duration {
        let secs = duration.as_secs_f64();
        if 1e-9 <= secs && secs < 1e-6 {
            write!(output, "{:5.1} ns", secs / 1e-9).unwrap();
        } else if 1e-6 <= secs && secs < 1e-3 {
            write!(output, "{:5.1} μs", secs / 1e-6).unwrap();
        } else if 1e-3 <= secs && secs < 1.0 {
            write!(output, "{:5.1} ms", secs / 1e-3).unwrap();
        } else if 1.0 <= secs && secs < 1e3 || secs == 0.0 {
            write!(output, "{:5.1} s ", secs).unwrap();
        } else {
            write!(output, "{:5.1e} s ", secs).unwrap();
        }
    } else {
        write!(output, "   N/A   ").unwrap();
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
        Self {
            span_stats: Default::default(),
        }
    }

    pub fn merge_with_others<'a>(&mut self, others: impl Iterator<Item = &'a AccumulatedTimings>) {
        for other in others {
            for (path, stats) in &other.span_stats {
                let current_stats = self.span_stats.entry(path.clone()).or_default();
                current_stats.combine_mut(&stats);
            }
        }
    }
}

impl AccumulatedTimings {
    pub fn create_timing_tree(&self) -> TimingTree {
        // The path entries present in the map might not form a valid span tree.
        // Therefore, we have to ensure that:
        //  - there's a root node
        //  - that every node except the root has its parent also present in the tree
        //  - there are no duplicate nodes
        //  - the paths are sorted depth-first

        let mut map: HashMap<_, _> = self
            .span_stats
            .iter()
            .map(|(path, stats)| (path.clone(), Some(stats.clone())))
            .collect();

        // The root node is the common ancestor of all the paths
        let common_ancestor = self
            .span_stats
            .keys()
            // TODO: This can be done much more efficiently with some manual labor
            // (i.e. start with the first element and keep knocking off names
            // so that the path is an ancestor of *all* paths)
            .fold(None, |common: Option<SpanPath>, path| match common {
                None => Some(path.clone()),
                Some(current_common) => Some(current_common.common_ancestor(path)),
            });

        if let Some(common_ancestor) = common_ancestor {
            // Insert all "intermediate nodes". For example, if the hash map contains
            // a>b>c, then try to insert a>b and a, provided they don't "extend past"
            // the common ancestor
            for mut path in self.span_stats.keys().cloned() {
                while let Some(parent_path) = path.parent() {
                    if parent_path.depth() < common_ancestor.depth() {
                        break;
                    } else {
                        if !map.contains_key(&parent_path) {
                            map.insert(parent_path.clone(), None);
                        }
                        path = parent_path;
                    }
                }
            }

            // The paths may form a forest, not a tree. We therefore insert the common
            // ancestor, which will function as the root of the tree.
            map.entry(common_ancestor).or_insert(None);
        }

        let mut path_duration_pairs: Vec<_> = map.into_iter().collect();

        path_duration_pairs.sort_by(|pair1, pair2| pair1.0.span_names().cmp(pair2.0.span_names()));
        let (paths_depth_first, durations) = path_duration_pairs.into_iter().unzip();

        SpanTree::try_from_depth_first_ordering(paths_depth_first, durations)
            .expect("Input should always be a valid span tree")
            .transform_payloads(|node| {
                node.payload().as_ref().map(|stats| {
                    let duration = stats.duration;
                    DerivedStats {
                        duration: stats.duration,
                        count: stats.count,
                        duration_relative_to_parent: node.parent().and_then(|parent_node| {
                            parent_node.payload().as_ref().map(|parent_stats| {
                                let parent_duration = parent_stats.duration;
                                let proportion = duration.as_secs_f64() / parent_duration.as_secs_f64();
                                proportion
                            })
                        }),
                        duration_relative_to_root: node.root().payload().as_ref().map(|root_stats| {
                            let root_duration = root_stats.duration;
                            let proportion = duration.as_secs_f64() / root_duration.as_secs_f64();
                            proportion
                        }),
                    }
                })
            })
    }
}

#[derive(Debug, Clone)]
pub struct AccumulatedTimingSeries {
    steps: Vec<AccumulatedStepTimings>,
    /// Timings for any spans that are not part of the "step" span (could be related to setup)
    /// or similar.
    intransient_timings: AccumulatedTimings,
    // TODO: Timing from other sources outside of steps?
}

impl AccumulatedTimingSeries {
    pub fn summarize(&self) -> AccumulatedTimings {
        let mut summary = self.intransient_timings.clone();
        summary.merge_with_others(self.steps().iter().map(|step| &step.timings));
        summary
    }
}

impl AccumulatedTimingSeries {
    pub fn steps(&self) -> &[AccumulatedStepTimings] {
        &self.steps
    }
}

pub fn extract_step_timings<'a>(records: impl IntoIterator<Item = Record>) -> eyre::Result<AccumulatedTimingSeries> {
    // TODO: Collect statistics from spans outside run as well
    find_and_visit_dynamecs_run_span(records.into_iter())
}

pub fn extract_timing_summary<'a>(records: impl IntoIterator<Item = Record>) -> eyre::Result<AccumulatedTimings> {
    extract_step_timings(records).map(|series| series.summarize())
}

fn find_and_visit_dynamecs_run_span<'a>(
    mut records: impl Iterator<Item = Record>,
) -> eyre::Result<AccumulatedTimingSeries> {
    // First try to find the `run` span in the records
    while let Some(record) = records.next() {
        if let Some(span) = record.span() {
            if span.name() == "run" && record.target() == "dynamecs_app" && record.kind() == RecordKind::SpanEnter {
                return visit_dynamecs_run_span(&record, records);
            }
        }
    }

    Err(eyre!(
        "Could not find new event for `run` span of dynamecs among records"
    ))
}

fn visit_dynamecs_run_span<'a>(
    run_new_record: &Record,
    remaining_records: impl Iterator<Item = Record>,
) -> eyre::Result<AccumulatedTimingSeries> {
    let run_thread = run_new_record.thread_id();
    let mut iter = remaining_records;
    let mut steps = Vec::new();

    let mut intransient_accumulator = TimingAccumulator::new();
    intransient_accumulator.enter_span(run_new_record.create_span_path()?, *run_new_record.timestamp())?;

    while let Some(record) = iter.next() {
        if record.thread_id() == run_thread {
            if let Some(span) = record.span() {
                match (span.name(), record.target(), record.kind()) {
                    ("step", "dynamecs_app", SpanEnter) => {
                        if let Some(step) = visit_dynamecs_step_span(&record, &mut iter)? {
                            // Only collect complete time steps
                            steps.push(step);
                        }
                    }
                    // Accumulate "intransient timings", i.e. timings for things that are
                    // not inside of a step
                    (_, _, SpanEnter) => {
                        intransient_accumulator.enter_span(record.create_span_path()?, *record.timestamp())?
                    }
                    (span_name, record_target, SpanExit) => {
                        intransient_accumulator.exit_span(record.create_span_path()?, *record.timestamp())?;
                        if span_name == "run" && record_target == "dynamecs_app" {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(AccumulatedTimingSeries {
        steps,
        intransient_timings: AccumulatedTimings {
            span_stats: intransient_accumulator.collect_completed_statistics(),
        },
    })
}

/// Returns accumulated timings for the next *complete* step in the records.
fn visit_dynamecs_step_span<'a>(
    step_new_record: &Record,
    remaining_records: &mut impl Iterator<Item = Record>,
) -> eyre::Result<Option<AccumulatedStepTimings>> {
    let step_path = step_new_record.create_span_path()?;

    let mut accumulator = TimingAccumulator::new();
    accumulator.enter_span(step_path.clone(), step_new_record.timestamp().clone())?;

    let step_index = step_new_record
        .span()
        .and_then(|span| span.fields().pointer("/step_index"))
        .and_then(|value| value.as_u64())
        .ok_or_else(|| eyre!("step span does not have step_index field"))?;

    while let Some(record) = remaining_records.next() {
        if record.thread_id() == step_new_record.thread_id() {
            if let Some(span) = record.span() {
                match record.kind() {
                    SpanEnter => {
                        accumulator.enter_span(record.create_span_path()?, record.timestamp().clone())?;
                    }
                    SpanExit => {
                        // TODO: use a stack to verify that open/close events are consistent?
                        let span_path = record.create_span_path()?;
                        let is_step_span_path = span_path == step_path;
                        accumulator.exit_span(span_path, record.timestamp().clone())?;
                        if span.name() == "step" && record.target() == "dynamecs_app" && is_step_span_path {
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
            timings: AccumulatedTimings {
                span_stats: accumulator.collect_completed_statistics(),
            },
            step_index,
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
        Self {
            completed_statistics: Default::default(),
            enter_timestamps: Default::default(),
        }
    }

    pub fn enter_span(&mut self, path: SpanPath, timestamp: OffsetDateTime) -> eyre::Result<()> {
        match self.enter_timestamps.entry(path) {
            Entry::Vacant(vacancy) => {
                vacancy.insert(timestamp);
                Ok(())
            }
            Entry::Occupied(old) => Err(eyre!(
                "tried to create new span {} that is already active\
                                               (not closed)",
                old.key()
            )),
        }
    }

    pub fn exit_span(&mut self, path: SpanPath, timestamp_close: OffsetDateTime) -> eyre::Result<()> {
        let timestamp_enter = self
            .enter_timestamps
            .remove(&path)
            .ok_or_else(|| eyre!("found close event for span that is not currently active. Span path: {path}"))?;
        let span_duration: Duration = (timestamp_close - timestamp_enter).unsigned_abs();
        let accumulated_stats = self.completed_statistics.entry(path).or_default();
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
