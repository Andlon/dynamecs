use serde_json::json;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use dynamecs_analyze::{iterate_records_from_reader, Level, Record, RecordKind};

#[test]
fn test_basic_records_iteration() {
    let log_data = r###"
        {"timestamp":"2023-03-29T12:48:50.213348Z","level":"TRACE","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}]}
        {"timestamp":"2023-03-29T12:48:51.440914Z","level":"INFO","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"hessian"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}]}
        {"timestamp":"2023-03-29T12:48:51.440972Z","level":"TRACE","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"}]}
        {"timestamp":"2023-03-29T12:48:51.441519Z","level":"DEBUG","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"solve_linear_system"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"solve_linear_system"}]}
    "###;
    let records: Vec<Record> = iterate_records_from_reader(log_data.as_bytes()).unwrap()
        .collect::<eyre::Result<_>>()
        .unwrap();

    assert_eq!(records.len(), 4);

    {
        let record = &records[0];
        assert_eq!(record.level(), Level::Trace);
        assert_eq!(record.target(), "dynsys::backward_euler");
        assert_eq!(record.kind(), RecordKind::SpanEnter);
        assert_eq!(record.message(), "enter");
        assert_eq!(record.timestamp(), &OffsetDateTime::parse("2023-03-29T12:48:50.213348Z", &Iso8601::DEFAULT).unwrap());
        assert_eq!(record.span().name(), "Backward Euler IP assemble");
        assert_eq!(record.span().fields(), &json! {{
            "name": "Backward Euler IP assemble",
        }});
        assert_eq!(record.spans().len(), 6);
        assert_eq!(record.spans()[0].name(), "run");
        assert_eq!(record.spans()[0].fields(), &json! {{
            "name": "run"
        }});
        assert_eq!(record.spans()[1].name(), "step");
        assert_eq!(record.spans()[1].fields(), &json! {{
            "name": "step",
            "step_index": 16
        }});
        assert_eq!(record.spans()[2].name(), "Backward Euler");
        assert_eq!(record.spans()[2].fields(), &json! {{
            "name": "Backward Euler"
        }});
        assert_eq!(record.spans()[3].name(), "Backward Euler");
        assert_eq!(record.spans()[3].fields(), &json! {{
            "name": "Backward Euler"
        }});
        assert_eq!(record.spans()[4].name(), "Newton iteration");
        assert_eq!(record.spans()[4].fields(), &json! {{
            "name": "Newton iteration",
            "hessian_mod": "NoModification",
            "k": 8,
        }});
        assert_eq!(record.spans()[5].name(), "Backward Euler IP assemble");
        assert_eq!(record.spans()[5].fields(), &json! {{
            "name": "Backward Euler IP assemble",
        }});
    }

    {
        let record = &records[1];
        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.target(), "dynsys::backward_euler");
        assert_eq!(record.kind(), RecordKind::SpanExit);
        assert_eq!(record.message(), "exit");
        assert_eq!(record.timestamp(), &OffsetDateTime::parse("2023-03-29T12:48:51.440914Z", &Iso8601::DEFAULT).unwrap());
        assert_eq!(record.span().name(), "hessian");
        assert_eq!(record.span().fields(), &json! {{
            "name": "hessian",
        }});
        assert_eq!(record.spans().len(), 6);

        // TODO: Test the rest
    }
}