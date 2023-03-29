use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::{Duration as StdDuration};
use eyre::eyre;
use time::OffsetDateTime;
use crate::{Record, RecordKind};

#[derive(Debug, Clone)]
pub struct SpanTiming {
    span_name: String,
    children: Vec<SpanTiming>,
    self_duration: StdDuration,
}

impl SpanTiming {
    pub fn name(&self) -> &str {
        &self.span_name
    }

    pub fn self_duration(&self) -> StdDuration {
        self.self_duration
    }

    pub fn total_duration(&self) -> StdDuration {
        let mut duration = self.self_duration;
        for child in &self.children {
            duration += child.total_duration();
        }
        duration
    }

    pub fn children(&self) -> &[SpanTiming] {
        &self.children
    }
}

pub fn accumulate_timings(records: impl IntoIterator<Item=Record>) -> eyre::Result<Vec<SpanTiming>> {
    accumulate_timings_(records.into_iter())
}

fn accumulate_timings_(records: impl Iterator<Item=Record>) -> eyre::Result<Vec<SpanTiming>> {
    let mut span_timer = SpanTimer::default();
    for record in records {
        if [RecordKind::SpanEnter, RecordKind::SpanExit].contains(&record.kind()) {
            let mut span_names = record.spans().iter().map(|span| span.name().to_string()).collect();
            match record.kind {
                RecordKind::SpanEnter => span_timer.enter_span(span_names, *record.timestamp())?,
                RecordKind::SpanExit => {
                    // Exit records do not record their own span in their list of spans,
                    // so we must add this to the span list in order to match up with
                    // the enter record
                    span_names.push(record.span().name().to_string());
                    span_timer.exit_span(span_names, *record.timestamp())?
                },
                RecordKind::Event => {}
            }
        }
    }
    Ok(span_timer.collect_accumulated_timings())
}

#[derive(Default, Debug)]
struct SpanTimer {
    accumulated_span_timings: HashMap<Vec<String>, StdDuration>,
    open_span_enter_timestamps: HashMap<Vec<String>, OffsetDateTime>
}

impl SpanTimer {
    fn enter_span(&mut self, spans: Vec<String>, timestamp: OffsetDateTime) -> eyre::Result<()> {
        let entry = self.open_span_enter_timestamps.entry(spans);
        match entry {
            Entry::Occupied(_) => Err(eyre!("Tried to enter span that was already entered")),
            Entry::Vacant(entry) => {
                entry.insert(timestamp); Ok(())
            }
        }
    }

    fn exit_span(&mut self, spans: Vec<String>, timestamp: OffsetDateTime) -> eyre::Result<()> {
        let entry = self.open_span_enter_timestamps.entry(spans);
        match entry {
            Entry::Occupied(entry) => {
                let exit_timestamp = timestamp;
                let (spans, enter_timestamp) = entry.remove_entry();
                let duration: StdDuration = (exit_timestamp - enter_timestamp).unsigned_abs();
                let old_duration = self.accumulated_span_timings.entry(spans)
                    .or_insert_with(|| StdDuration::ZERO);
                *old_duration += duration;
                Ok(())
            },
            Entry::Vacant(_) => {
                Err(eyre!("Tried to exit span that was not already entered"))
            }
        }
    }

    fn collect_accumulated_timings(self) -> Vec<SpanTiming> {
        let mut entries: Vec<_> = self.accumulated_span_timings.into_iter()
            .collect();
        // By sorting the entries, we always obtain parents before child entries
        entries.sort();
        let mut timings = Vec::new();

        for (span_strings, duration) in entries {
            let timing = SpanTiming {
                span_name: span_strings.last().cloned().unwrap_or_else(|| String::new()),
                children: vec![],
                self_duration: duration
            };
            if span_strings.is_empty() {
                timings.push(timing);
            } else {
                insert_in_parent(timing, &span_strings[..span_strings.len() - 1], &mut timings);
            }
        }

        timings
    }
}

fn insert_in_parent(
    timing: SpanTiming,
    parent_strings: &[String],
    potential_ancestors: &mut Vec<SpanTiming>
) {
    if let Some(oldest_ancestor_name) = parent_strings.first() {
        if let Some(oldest_ancestor) = potential_ancestors.iter_mut().find(|span| span.name() == oldest_ancestor_name) {
            insert_in_parent(timing, &parent_strings[1..], &mut oldest_ancestor.children);
        } else {
            // There is no such ancestor, so let's create one
            // This shouldn't normally happen but could perhaps happen if for some reason
            // spans are mismatched in the log
            let mut ancestor = SpanTiming {
                span_name: oldest_ancestor_name.clone(),
                children: vec![],
                self_duration: StdDuration::ZERO,
            };
            insert_in_parent(timing, &parent_strings[1..], &mut ancestor.children);
        }
    } else {
        // the entry has no parent, so it must be a sibling of the potential ancestors
        potential_ancestors.push(timing)
    }
}