use eyre::{eyre, ErrReport};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Lines, Read, Write};
use std::path::Path;
use std::str::FromStr;
use time::OffsetDateTime;

pub mod timing;

mod span_path;
pub use span_path::SpanPath;

mod span_tree;
pub use span_tree::{SpanTree, SpanTreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    name: String,
    // TODO: Replace with Map<String, Value>, since we always expect it to be an object?
    fields: serde_json::Value,
}

impl Span {
    pub fn from_name_and_fields(name: impl Into<String>, fields: serde_json::Value) -> Self {
        let mut fields = fields;
        let name = name.into();
        // fields must contain "name" as a field. Unfortunately tracing puts the name
        // of the span and its fields in the same object, which might lead to collisions...
        // The only way to fix this would be to use our own JSON format, which we will probably
        // do some time in the future.
        fields
            .as_object_mut()
            .expect("fields must be a JSON object")
            .insert("name".to_string(), serde_json::Value::String(name.clone()));
        Self { name, fields }
    }

    fn try_from_json_value(value: serde_json::Value) -> eyre::Result<Self> {
        let name = value
            .as_object()
            .and_then(|obj| obj.get("name").and_then(|val| val.as_str()))
            .ok_or_else(|| eyre!("missing name in span"))?
            .to_string();
        Ok(Self { name, fields: value })
    }

    fn to_json_value(self) -> serde_json::Value {
        let mut fields = self.fields;
        *fields.get_mut("name").unwrap() = Value::String(self.name);
        fields
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    target: String,
    span: Option<Span>,
    level: Level,
    spans: Option<Vec<Span>>,
    kind: RecordKind,
    message: Option<String>,
    timestamp: OffsetDateTime,
    thread_id: String,
    fields: serde_json::Value,
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

    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(AsRef::as_ref)
    }

    pub fn kind(&self) -> RecordKind {
        self.kind
    }

    pub fn timestamp(&self) -> &OffsetDateTime {
        &self.timestamp
    }

    /// Create the span path associated with this record.
    ///
    /// For span enter/exit records, this is the span that is currently being entered/exited,
    /// and for events it is the path to the span in which the event takes place.
    pub fn create_span_path(&self) -> eyre::Result<SpanPath> {
        let mut span_names: Vec<_> = self
            .spans
            .iter()
            .flatten()
            .map(|span| span.name.clone())
            .collect();
        match self.kind() {
            RecordKind::SpanEnter | RecordKind::Event => {}
            RecordKind::SpanExit => {
                // The exit record does not include the span currently being exited
                // in the list of entered spans.
                let span_name = self
                    .span()
                    .map(|span| span.name())
                    .ok_or_else(|| eyre!("No span in exit record"))?;
                span_names.push(span_name.to_string());
            }
        }
        Ok(SpanPath::new(span_names))
    }

    pub fn thread_id(&self) -> &str {
        &self.thread_id
    }

    pub fn fields(&self) -> &serde_json::Value {
        &self.fields
    }
}

#[derive(Default, Debug, Clone)]
pub struct RecordBuilder {
    target: Option<String>,
    span: Option<Span>,
    level: Option<Level>,
    spans: Option<Vec<Span>>,
    kind: Option<RecordKind>,
    message: Option<String>,
    timestamp: Option<OffsetDateTime>,
    thread_id: Option<String>,
    fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct RecordBuildError {
    message: String,
}

impl Display for RecordBuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error building record: {}", &self.message)
    }
}

impl RecordBuildError {
    fn missing_field(field_name: &str) -> Self {
        Self {
            message: format!("missing field {field_name} in Record construction"),
        }
    }

    fn message(message: String) -> Self {
        Self { message }
    }
}

impl RecordBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_record(record: Record) -> Self {
        Self {
            target: Some(record.target),
            span: record.span,
            level: Some(record.level),
            spans: record.spans,
            kind: Some(record.kind),
            message: record.message,
            timestamp: Some(record.timestamp),
            thread_id: Some(record.thread_id),
            fields: Some(record.fields),
        }
    }

    pub fn event() -> Self {
        Self {
            kind: Some(RecordKind::Event),
            ..Self::default()
        }
    }

    pub fn span_enter() -> Self {
        Self {
            kind: Some(RecordKind::SpanEnter),
            ..Self::default()
        }
    }

    pub fn span_exit() -> Self {
        Self {
            kind: Some(RecordKind::SpanExit),
            ..Self::default()
        }
    }

    pub fn info(self) -> Self {
        Self {
            level: Some(Level::Info),
            ..self
        }
    }

    pub fn warn(self) -> Self {
        Self {
            level: Some(Level::Warn),
            ..self
        }
    }

    pub fn debug(self) -> Self {
        Self {
            level: Some(Level::Debug),
            ..self
        }
    }

    pub fn trace(self) -> Self {
        Self {
            level: Some(Level::Trace),
            ..self
        }
    }

    pub fn error(self) -> Self {
        Self {
            level: Some(Level::Error),
            ..self
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target.replace(target.into());
        self
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span.replace(span);
        self
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level.replace(level);
        self
    }

    pub fn spans(mut self, spans: Vec<Span>) -> Self {
        self.spans.replace(spans);
        self
    }

    pub fn kind(mut self, kind: RecordKind) -> Self {
        self.kind.replace(kind);
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message.replace(message.into());
        self
    }

    pub fn timestamp(mut self, timestamp: OffsetDateTime) -> Self {
        self.timestamp.replace(timestamp);
        self
    }

    pub fn thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id.replace(thread_id.into());
        self
    }

    pub fn fields(mut self, fields: serde_json::Value) -> Self {
        self.fields.replace(fields);
        self
    }

    pub fn try_build(self) -> Result<Record, RecordBuildError> {
        let kind = self
            .kind
            .ok_or_else(|| RecordBuildError::missing_field("kind"))?;

        let message = match kind {
            RecordKind::SpanEnter => {
                let msg_valid = self.message.map(|msg| msg == "enter").unwrap_or(true);
                if !msg_valid {
                    return Err(RecordBuildError::message(
                        "span enter records cannot have \
                             message other than \"enter\""
                            .to_string(),
                    ));
                }
                Some("enter".to_string())
            }
            RecordKind::SpanExit => {
                let msg_valid = self.message.map(|msg| msg == "exit").unwrap_or(true);
                if !msg_valid {
                    return Err(RecordBuildError::message(
                        "span exit records cannot have \
                             message other than \"exit\""
                            .to_string(),
                    ));
                }
                Some("exit".to_string())
            }
            RecordKind::Event => self.message,
        };

        Ok(Record {
            target: self
                .target
                .ok_or_else(|| RecordBuildError::missing_field("target"))?,
            span: self.span,
            level: self
                .level
                .ok_or_else(|| RecordBuildError::missing_field("level"))?,
            spans: self.spans,
            kind,
            timestamp: self
                .timestamp
                .ok_or_else(|| RecordBuildError::missing_field("timestamp"))?,
            thread_id: self
                .thread_id
                .ok_or_else(|| RecordBuildError::missing_field("thread_id"))?,
            fields: {
                let mut fields = self
                    .fields
                    .unwrap_or_else(|| serde_json::Value::Object(Map::default()));
                if let Some(message) = &message {
                    fields
                        .as_object_mut()
                        .expect("Fields must be a JSON object")
                        .insert("message".to_string(), serde_json::Value::String(message.clone()));
                }
                fields
            },
            message,
        })
    }

    pub fn build(self) -> Record {
        self.try_build().unwrap()
    }
}

pub struct RecordIter<'a> {
    lines_iter: Lines<BufReader<Box<dyn Read + 'a>>>,
}

pub fn iterate_records(json_log_file_path: impl AsRef<Path>) -> eyre::Result<RecordIter<'static>> {
    iterate_records_(json_log_file_path.as_ref())
}

fn iterate_records_(json_log_file_path: &Path) -> eyre::Result<RecordIter<'static>> {
    let file = File::open(json_log_file_path)?;
    let file_name = json_log_file_path
        .file_name()
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

pub fn iterate_records_from_reader<'a, R: Read + 'a>(reader: R) -> RecordIter<'a> {
    iterate_records_from_reader_(BufReader::new(Box::new(reader)))
}

fn iterate_records_from_reader_<'a>(reader: BufReader<Box<dyn Read + 'a>>) -> RecordIter<'a> {
    RecordIter {
        lines_iter: reader.lines(),
    }
}

pub fn write_records(mut writer: impl Write, records: impl Iterator<Item = Record>) -> io::Result<()> {
    for record in records {
        let raw_record = RawRecord::from_record(record);
        serde_json::to_writer(&mut writer, &raw_record)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

impl<'a> Iterator for RecordIter<'a> {
    // TODO: Use a proper error type here
    type Item = eyre::Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(line_result) = self.lines_iter.next() {
            match line_result {
                Ok(line) if line.trim().is_empty() => {}
                Ok(line) => {
                    return Some(
                        serde_json::from_str(&line)
                            .map_err(|err| ErrReport::from(err))
                            .and_then(|raw_record: RawRecord| raw_record.try_to_record()),
                    )
                }
                Err(err) => {
                    return Some(Err(err.into()));
                }
            }
        }

        None
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawRecord {
    // TODO: Consider replacing time with Chrono. From my understanding, only Chrono
    // properly and soundly works with local time on Linux
    #[serde(with = "time::serde::rfc3339")]
    timestamp: OffsetDateTime,
    level: String,
    fields: serde_json::Value,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spans: Option<Vec<serde_json::Value>>,
    #[serde(rename = "threadId")]
    thread_id: String,
}

impl RawRecord {
    fn try_to_record(self) -> eyre::Result<Record> {
        let message = self.fields.pointer("/message").and_then(|val| val.as_str());

        Ok(Record {
            target: self.target,
            span: self
                .span
                .map(|json_val| Span::try_from_json_value(json_val))
                .transpose()?,
            level: Level::from_str(&self.level)?,
            spans: self
                .spans
                .map(|json_vals| {
                    json_vals
                        .into_iter()
                        .map(Span::try_from_json_value)
                        .collect::<eyre::Result<_>>()
                })
                .transpose()?,
            kind: match message {
                Some(string) if string == "enter" => RecordKind::SpanEnter,
                Some(string) if string == "exit" => RecordKind::SpanExit,
                _ => RecordKind::Event,
            },
            message: message.map(str::to_string),
            timestamp: self.timestamp,
            thread_id: self.thread_id,
            fields: self.fields,
        })
    }

    fn from_record(record: Record) -> Self {
        let mut fields = record.fields;

        let mut message = record.message;
        match record.kind {
            RecordKind::SpanEnter => {
                message.replace("enter".to_string());
            }
            RecordKind::SpanExit => {
                message.replace("exit".to_string());
            }
            RecordKind::Event => {}
        }

        if let Some(message) = message {
            fields
                .as_object_mut()
                .expect("Fields must always have object type")
                .insert("message".to_string(), Value::String(message));
        }

        Self {
            timestamp: record.timestamp,
            level: record.level.to_string(),
            fields,
            target: record.target,
            span: record.span.map(|span| span.to_json_value()),
            spans: record
                .spans
                .map(|spans| spans.into_iter().map(|span| span.to_json_value()).collect()),
            thread_id: record.thread_id,
        }
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
    Trace,
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

impl ToString for Level {
    fn to_string(&self) -> String {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
        .to_string()
    }
}

// #[macro_export]
// macro_rules! record {
//     ($)
//     // ($($field:ident = $val:expr),*) => {
//     //     {
//     //         $crate::RecordBuilder::new()
//     //             $(.with_$field($val))*
//     //             .build()
//     //     }
//     // }
// }
