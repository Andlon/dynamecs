use dynamecs_analyze::{iterate_records_from_reader, write_records, Level, Record, RecordBuilder, RecordKind, Span};
use serde_json::json;
use serde_json::Value::Object;
use std::error::Error;
use time::format_description::well_known::Iso8601;
use time::Month::February;
use time::{Date, Duration, OffsetDateTime, UtcOffset};

/// Helper macro for succinctly creating a span path from a list of literals.
macro_rules! span_path {
    ($($strings:expr),*) => {
        SpanPath::new(vec![$($strings.to_string()),*])
    }
}

mod span_path;
mod span_tree;
mod timing;

#[test]
fn test_basic_records_iteration() {
    let log_data = r###"
        {"timestamp":"2023-03-29T12:48:50.213348Z","level":"TRACE","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}], "threadId": "ThreadId(0)"}
        {"timestamp":"2023-03-29T12:48:51.440914Z","level":"INFO","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"hessian"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}], "threadId": "ThreadId(0)"}
        {"timestamp":"2023-03-29T12:48:51.440972Z","level":"TRACE","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"}], "threadId": "ThreadId(0)"}
        {"timestamp":"2023-03-29T12:48:51.441519Z","level":"DEBUG","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"solve_linear_system"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"solve_linear_system"}], "threadId": "ThreadId(0)"}
    "###;
    let records: Vec<Record> = iterate_records_from_reader(log_data.as_bytes())
        .collect::<eyre::Result<_>>()
        .unwrap();

    assert_eq!(records.len(), 4);

    {
        let record = &records[0];
        assert_eq!(record.level(), Level::Trace);
        assert_eq!(record.target(), "dynsys::backward_euler");
        assert_eq!(record.kind(), RecordKind::SpanEnter);
        assert_eq!(record.message(), Some("enter"));
        assert_eq!(
            record.timestamp(),
            &OffsetDateTime::parse("2023-03-29T12:48:50.213348Z", &Iso8601::DEFAULT).unwrap()
        );
        assert_eq!(record.span().unwrap().name(), "Backward Euler IP assemble");
        assert_eq!(
            record.span().unwrap().fields(),
            &json! {{
                "name": "Backward Euler IP assemble",
            }}
        );
        assert_eq!(record.spans().unwrap().len(), 6);
        assert_eq!(record.spans().unwrap()[0].name(), "run");
        assert_eq!(
            record.spans().unwrap()[0].fields(),
            &json! {{
                "name": "run"
            }}
        );
        assert_eq!(record.spans().unwrap()[1].name(), "step");
        assert_eq!(
            record.spans().unwrap()[1].fields(),
            &json! {{
                "name": "step",
                "step_index": 16
            }}
        );
        assert_eq!(record.spans().unwrap()[2].name(), "Backward Euler");
        assert_eq!(
            record.spans().unwrap()[2].fields(),
            &json! {{
                "name": "Backward Euler"
            }}
        );
        assert_eq!(record.spans().unwrap()[3].name(), "Backward Euler");
        assert_eq!(
            record.spans().unwrap()[3].fields(),
            &json! {{
                "name": "Backward Euler"
            }}
        );
        assert_eq!(record.spans().unwrap()[4].name(), "Newton iteration");
        assert_eq!(
            record.spans().unwrap()[4].fields(),
            &json! {{
                "name": "Newton iteration",
                "hessian_mod": "NoModification",
                "k": 8,
            }}
        );
        assert_eq!(record.spans().unwrap()[5].name(), "Backward Euler IP assemble");
        assert_eq!(
            record.spans().unwrap()[5].fields(),
            &json! {{
                "name": "Backward Euler IP assemble",
            }}
        );
    }

    {
        let record = &records[1];
        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.target(), "dynsys::backward_euler");
        assert_eq!(record.kind(), RecordKind::SpanExit);
        assert_eq!(record.message(), Some("exit"));
        assert_eq!(
            record.timestamp(),
            &OffsetDateTime::parse("2023-03-29T12:48:51.440914Z", &Iso8601::DEFAULT).unwrap()
        );
        assert_eq!(record.span().unwrap().name(), "hessian");
        assert_eq!(
            record.span().unwrap().fields(),
            &json! {{
                "name": "hessian",
            }}
        );
        assert_eq!(record.spans().unwrap().len(), 6);

        // TODO: Test the rest
    }
}

pub struct IncrementalTimestamp {
    timestamp: OffsetDateTime,
}

impl IncrementalTimestamp {
    pub fn current(&self) -> OffsetDateTime {
        self.timestamp
    }

    /// Advance the time by the given duration and return the result.
    pub fn advance_by(&mut self, duration: Duration) -> OffsetDateTime {
        self.timestamp += duration;
        self.timestamp
    }
}

impl Default for IncrementalTimestamp {
    fn default() -> Self {
        let base_date = Date::from_calendar_date(2023, February, 22)
            .unwrap()
            .with_hms(08, 00, 00)
            .unwrap()
            // Use a timezone different than UTC
            .assume_offset(UtcOffset::from_hms(02, 00, 00).unwrap());
        Self { timestamp: base_date }
    }
}

#[test]
fn test_write_records() -> Result<(), Box<dyn Error>> {
    let mut next_date = IncrementalTimestamp::default();

    {
        let records = vec![
            RecordBuilder::event()
                .info()
                .target("a")
                .message("msg0")
                .thread_id("0")
                .timestamp(next_date.current())
                .build(),
            RecordBuilder::event()
                .trace()
                .target("a")
                .message("msg1")
                .timestamp(next_date.advance_by(Duration::seconds(1)))
                .thread_id("0")
                .build(),
            RecordBuilder::span_enter()
                .info()
                .target("b")
                .timestamp(next_date.advance_by(Duration::seconds(1)))
                .thread_id("0")
                .span(Span::from_name_and_fields("span1", Object(Default::default())))
                .spans(vec![Span::from_name_and_fields("span1", Object(Default::default()))])
                .build(),
            RecordBuilder::event()
                .debug()
                .target("b")
                .timestamp(next_date.advance_by(Duration::seconds(1)))
                .thread_id("1")
                .message("message2")
                .span(Span::from_name_and_fields("span1", Object(Default::default())))
                .spans(vec![Span::from_name_and_fields("span1", Object(Default::default()))])
                .build(),
            RecordBuilder::event()
                .warn()
                .target("b")
                .timestamp(next_date.advance_by(Duration::seconds(1)))
                .thread_id("0")
                .fields(json!( { "field1": 4, "field2": "value2" }))
                .span(Span::from_name_and_fields("span1", Object(Default::default())))
                .spans(vec![Span::from_name_and_fields("span1", Object(Default::default()))])
                .build(),
            RecordBuilder::span_exit()
                .info()
                .target("b")
                .timestamp(next_date.advance_by(Duration::seconds(1)))
                .thread_id("0")
                .span(Span::from_name_and_fields("span1", Object(Default::default())))
                .build(),
        ];

        let mut records_bytes: Vec<u8> = Vec::new();
        write_records(&mut records_bytes, records.into_iter())?;
        let records_string = String::from_utf8(records_bytes).unwrap();

        insta::assert_snapshot!(records_string);
    }

    Ok(())
}
