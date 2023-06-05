use std::error::Error;
use serde_json::json;
use time::format_description::well_known::Iso8601;
use time::{Date, Duration, OffsetDateTime, UtcOffset};
use time::Month::February;
use dynamecs_analyze::{iterate_records_from_reader, Level, Record, RecordBuilder, RecordKind, Span, write_records};

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
        assert_eq!(record.timestamp(), &OffsetDateTime::parse("2023-03-29T12:48:50.213348Z", &Iso8601::DEFAULT).unwrap());
        assert_eq!(record.span().unwrap().name(), "Backward Euler IP assemble");
        assert_eq!(record.span().unwrap().fields(), &json! {{
            "name": "Backward Euler IP assemble",
        }});
        assert_eq!(record.spans().unwrap().len(), 6);
        assert_eq!(record.spans().unwrap()[0].name(), "run");
        assert_eq!(record.spans().unwrap()[0].fields(), &json! {{
            "name": "run"
        }});
        assert_eq!(record.spans().unwrap()[1].name(), "step");
        assert_eq!(record.spans().unwrap()[1].fields(), &json! {{
            "name": "step",
            "step_index": 16
        }});
        assert_eq!(record.spans().unwrap()[2].name(), "Backward Euler");
        assert_eq!(record.spans().unwrap()[2].fields(), &json! {{
            "name": "Backward Euler"
        }});
        assert_eq!(record.spans().unwrap()[3].name(), "Backward Euler");
        assert_eq!(record.spans().unwrap()[3].fields(), &json! {{
            "name": "Backward Euler"
        }});
        assert_eq!(record.spans().unwrap()[4].name(), "Newton iteration");
        assert_eq!(record.spans().unwrap()[4].fields(), &json! {{
            "name": "Newton iteration",
            "hessian_mod": "NoModification",
            "k": 8,
        }});
        assert_eq!(record.spans().unwrap()[5].name(), "Backward Euler IP assemble");
        assert_eq!(record.spans().unwrap()[5].fields(), &json! {{
            "name": "Backward Euler IP assemble",
        }});
    }

    {
        let record = &records[1];
        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.target(), "dynsys::backward_euler");
        assert_eq!(record.kind(), RecordKind::SpanExit);
        assert_eq!(record.message(), Some("exit"));
        assert_eq!(record.timestamp(), &OffsetDateTime::parse("2023-03-29T12:48:51.440914Z", &Iso8601::DEFAULT).unwrap());
        assert_eq!(record.span().unwrap().name(), "hessian");
        assert_eq!(record.span().unwrap().fields(), &json! {{
            "name": "hessian",
        }});
        assert_eq!(record.spans().unwrap().len(), 6);

        // TODO: Test the rest
    }
}

#[test]
fn test_write_records() -> Result<(), Box<dyn Error>> {
    let base_date = Date::from_calendar_date(2023, February, 22)?.with_hms(08, 00, 00)?
        // Use a timezone different than UTC
        .assume_offset(UtcOffset::from_hms(02, 00, 00).unwrap());

    let mut next_date = base_date;
    let mut next_date = |increment: Duration| {
        next_date += increment;
        next_date.clone()
    };

    {
        let records = vec![
            RecordBuilder::new()
                .with_target("a")
                .with_message("msg0")
                .with_thread_id("0")
                .with_kind(RecordKind::Event)
                .with_level(Level::Info)
                .with_timestamp(base_date)
                .build(),
            RecordBuilder::new()
                .with_target("a")
                .with_message("msg1")
                .with_kind(RecordKind::Event)
                .with_level(Level::Trace)
                .with_timestamp(next_date(Duration::seconds(1)))
                .with_thread_id("0")
                .build(),
            RecordBuilder::new()
                .with_target("b")
                .with_kind(RecordKind::SpanEnter)
                .with_level(Level::Info)
                .with_timestamp(next_date(Duration::seconds(1)))
                .with_thread_id("0")
                .with_span(Span::from_name_and_fields("span1", serde_json::Value::Object(Default::default())))
                .with_spans(vec![Span::from_name_and_fields("span1", serde_json::Value::Object(Default::default()))])
                .build(),
            RecordBuilder::new()
                .with_target("b")
                .with_kind(RecordKind::SpanExit)
                .with_level(Level::Info)
                .with_timestamp(next_date(Duration::seconds(1)))
                .with_thread_id("0")
                .build()
        ];

        let mut records_bytes: Vec<u8> = Vec::new();
        write_records(&mut records_bytes, records.into_iter())?;

        // TODO: Use insta to verify that it looks as expected
        println!("{}", String::from_utf8(records_bytes).unwrap());
    }

    Ok(())
}