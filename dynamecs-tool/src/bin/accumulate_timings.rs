use std::collections::HashMap;
use std::error::Error;
use std::{fmt, slice};
use std::io::{stdout, Write};
use std::time::Duration;
use tabwriter::TabWriter;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::timing::{accumulate_timings, SpanTiming2};

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(arg) = std::env::args().skip(1).next() {
        let records: Vec<_> = iterate_records(&arg)?.collect::<Result<_, _>>()?;
        let timings = accumulate_timings(&records)?;

        // Map (parent, name) to durations, so that we can look up later
        let timings_map = compute_timings_map(&timings);
        let self_durations_map = compute_self_durations(&timings, &timings_map);

        let stdout = std::io::stdout().lock();
        let mut writer = TabWriter::new(stdout);
        writeln!(writer, "Duration\tSelf duration\tSelf %\t% of root\t% of parent\t Span")?;
        for timing in timings {
            let indent = {
                let pre_indent_num = timing.parent().len().saturating_sub(1);
                let mut indent = "│   ".repeat(pre_indent_num);
                if timing.parent().len() > 0 {
                    indent.push_str("├── ");
                }
                indent
            };
            let duration = format_duration(timing.duration());
            let root_ancestor = timing.parent().first();
            let root_duration = root_ancestor.and_then(|root| timings_map.get(slice::from_ref(root)).copied());
            let parent_duration = timings_map.get(timing.parent()).copied();
            let self_duration = self_durations_map.get(timing.span_path()).copied()
                .expect("Self duration must always be present");
            let self_duration_str = format_duration(self_duration);
            writeln!(writer, "{duration}\t{self_duration}\t{self_proportion}\t{root_proportion}\t{parent_proportion} \t {indent}{name}",
                     name = timing.name(),
                     self_duration = self_duration_str,
                     self_proportion = format_proportion(self_duration, Some(timing.duration())),
                     root_proportion = format_proportion(timing.duration(), root_duration),
                     parent_proportion = format_proportion(timing.duration(), parent_duration))?;
        }
        writer.flush()?;
    } else {
        eprintln!("No path to log file provided, exiting...");
    }
    Ok(())
}

fn compute_timings_map(timings: &[SpanTiming2]) -> HashMap<Vec<String>, Duration> {
    // Map (parent, name) to durations, so that we can look up later
    timings.iter()
        .map(|timing| {
            let mut path = timing.parent().to_vec();
            path.push(timing.name().to_string());
            (path, timing.duration())
        })
        .collect()
}

fn compute_self_durations(
    timings: &[SpanTiming2],
    timings_map: &HashMap<Vec<String>, Duration>
) -> HashMap<Vec<String>, Duration> {
    let mut self_durations_map = timings_map.clone();
    for timing in timings {
        if let Some(mut parent_self_duration) = self_durations_map.get_mut(timing.parent()) {
            *parent_self_duration -= timing.duration();
        }
    }
    self_durations_map
}

fn format_proportion(duration: Duration, other_duration: Option<Duration>) -> String {
    let proportion = other_duration.map(|relative_to| duration.as_secs_f64() / relative_to.as_secs_f64());
    proportion.map(|p| format!("{:5.1} %", p * 100.0))
        .unwrap_or_else(|| "N/A".to_string())
}

fn format_duration(duration: Duration) -> String {
    let s = duration.as_secs_f64();
    if s < 1e-7 {
        format!("{:7.2} ns", s * 1e9)
    } else if s < 1e-4 {
        format!("{:7.2} µs", s * 1e6)
    } else if s < 1e-1 {
        format!("{:7.2} ms", s * 1e3)
    } else {
        format!("{:7.2e}  s", s)
    }
}