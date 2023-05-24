use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, Read};
use std::path::Path;
use std::str::FromStr;
use eyre::{ErrReport, eyre};
use flate2::read::GzDecoder;
use serde::Deserialize;
use time::OffsetDateTime;

pub mod timing;
pub mod timing2;

mod span_path;
pub use span_path::{SpanPath};

mod span_tree;
pub use span_tree::{SpanTree, SpanTreeNode, InvalidTreeLayout};

#[derive(Debug, Clone)]
pub struct Span {
    name: String,
    fields: serde_json::Value
}

impl Span {
    fn try_from_json_value(value: serde_json::Value) -> eyre::Result<Self> {
        let name = value
            .as_object()
            .and_then(|obj| {
                obj.get("name")
                    .and_then(|val| val.as_str())
            })
            .ok_or_else(|| eyre!("missing name in span"))?
            .to_string();
        Ok(Self {
            name,
            fields: value
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &serde_json::Value {
        &self.fields
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    SpanEnter,
    SpanExit,
    Event,
}

#[derive(Debug, Clone)]
pub struct Record {
    target: String,
    span: Option<Span>,
    level: Level,
    spans: Option<Vec<Span>>,
    kind: RecordKind,
    message: String,
    timestamp: OffsetDateTime,
}

impl Record {
    pub fn level(&self) -> Level {
        self.level
    }

    pub fn span(&self) -> Option<&Span> {
        self.span.as_ref()
    }

    pub fn spans(&self) -> Option<&[Span]> {
        self.spans.as_ref().map(Vec::as_ref)
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn kind(&self) -> RecordKind {
        self.kind
    }

    pub fn timestamp(&self) -> &OffsetDateTime {
        &self.timestamp
    }

    pub fn span_path(&self) -> SpanPath {
        let span_names = self.spans
            .iter()
            .flatten()
            .map(|span| span.name.clone())
            .collect();
        SpanPath::new(span_names)
    }
}

pub struct RecordIter {
    lines_iter: Lines<BufReader<Box<dyn Read>>>,
}

pub fn iterate_records(json_log_file_path: impl AsRef<Path>) -> eyre::Result<RecordIter> {
    iterate_records_(json_log_file_path.as_ref())
}

fn iterate_records_(json_log_file_path: &Path) -> eyre::Result<RecordIter> {
    let file = File::open(json_log_file_path)?;
    let file_name = json_log_file_path.file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| eyre!("non-utf filename, cannot proceed"))?;
    if file_name.ends_with(".jsonlog") {
        Ok(iterate_records_from_reader(file))
    } else if file_name.ends_with(".jsonlog.gz") {
        Ok(iterate_records_from_reader(GzDecoder::new(file)))
    } else {
        Err(eyre!("unexpected extension. Expected .jsonlog or .jsonlog.gz"))
    }
}

pub fn iterate_records_from_reader<R: Read + 'static>(reader: R) -> RecordIter {
    iterate_records_from_reader_(BufReader::new(Box::new(reader)))
}

fn iterate_records_from_reader_(reader: BufReader<Box<dyn Read>>) -> RecordIter {
    RecordIter {
        lines_iter: reader.lines()
    }
}

impl Iterator for RecordIter {
    // TODO: Use a proper error type here
    type Item = eyre::Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line_result) = self.lines_iter.next() {
            match line_result {
                Ok(line) if line.trim().is_empty() => {},
                Ok(line) => {
                    return Some(serde_json::from_str(&line)
                        .map_err(|err| ErrReport::from(err))
                        .and_then(|raw_record: RawRecord| raw_record.try_to_record()))
                }
                Err(err) => {
                    return Some(Err(err.into()));
                }
            }
        }

        None
    }
}

#[derive(Debug, Deserialize)]
struct RawRecord {
    #[serde(with = "time::serde::iso8601")]
    timestamp: OffsetDateTime,
    level: String,
    fields: serde_json::Value,
    target: String,
    span: Option<serde_json::Value>,
    spans: Option<Vec<serde_json::Value>>,
}

impl RawRecord {
    fn try_to_record(self) -> eyre::Result<Record> {
        let message = self.fields.pointer("/message")
            .and_then(|val| val.as_str())
            .ok_or_else(|| eyre!("no message field"))?;

        Ok(Record {
            target: self.target,
            span: self.span.map(|json_val| Span::try_from_json_value(json_val)).transpose()?,
            level: Level::from_str(&self.level)?,
            spans: self.spans.map(|json_vals| json_vals.into_iter().map(Span::try_from_json_value).collect::<eyre::Result<_>>())
                .transpose()?,
            kind: match message {
                string if string == "enter" => RecordKind::SpanEnter,
                string if string == "exit" => RecordKind::SpanExit,
                _ => RecordKind::Event
            },
            message: message.to_string(),
            timestamp: self.timestamp,
        })
    }
}

// We reproduce a Level enum here so that we don't have to depend on tracing only for that one
// type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace
}

#[derive(Debug, Clone)]
pub struct InvalidLevelString;

impl std::error::Error for InvalidLevelString {}

impl Display for InvalidLevelString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid level")
    }
}

impl FromStr for Level {
    type Err = InvalidLevelString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("ERROR") {
            Ok(Self::Error)
        } else if trimmed.eq_ignore_ascii_case("WARN") {
            Ok(Self::Warn)
        } else if trimmed.eq_ignore_ascii_case("INFO") {
            Ok(Self::Info)
        } else if trimmed.eq_ignore_ascii_case("DEBUG") {
            Ok(Self::Debug)
        } else if trimmed.eq_ignore_ascii_case("TRACE") {
            Ok(Self::Trace)
        } else {
            Err(InvalidLevelString)
        }
    }
}