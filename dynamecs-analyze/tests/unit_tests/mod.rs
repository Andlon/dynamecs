use serde_json::json;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use dynamecs_analyze::{iterate_records_from_reader, Level, Record, RecordKind};
use dynamecs_analyze::timing::accumulate_timings;

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
        {"timestamp":"2023-03-29T12:48:50.213348Z","level":"TRACE","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}]}
        {"timestamp":"2023-03-29T12:48:51.440914Z","level":"INFO","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"hessian"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"Backward Euler IP assemble"}]}
        {"timestamp":"2023-03-29T12:48:51.440972Z","level":"TRACE","fields":{"message":"exit"},"target":"dynsys::backward_euler","span":{"name":"Backward Euler IP assemble"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"}]}
        {"timestamp":"2023-03-29T12:48:51.441519Z","level":"DEBUG","fields":{"message":"enter"},"target":"dynsys::backward_euler","span":{"name":"solve_linear_system"},"spans":[{"name":"run"},{"step_index":16,"name":"step"},{"name":"Backward Euler"},{"name":"Backward Euler"},{"hessian_mod":"NoModification","k":8,"name":"Newton iteration"},{"name":"solve_linear_system"}]}
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
fn test_basic_timing_collection() {
    let log_data = r###"
{"timestamp":"2023-03-29T18:16:50.073142Z","level":"INFO","fields":{"message":"enter"},"target":"dynamecs_app","span":{"name":"run"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.073564Z","level":"INFO","fields":{"message":"Starting simulation of scenario \"basic\""},"target":"dynamecs_app","span":{"name":"run"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.073929Z","level":"INFO","fields":{"message":"No time step configured. Using default dt = 0.016666666666666666"},"target":"dynamecs_app","span":{"name":"run"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.074164Z","level":"INFO","fields":{"message":"enter"},"target":"dynamecs_app","span":{"step_index":0,"name":"step"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.074370Z","level":"DEBUG","fields":{"message":"Running post-systems for initial state"},"target":"dynamecs_app","span":{"step_index":0,"name":"step"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.074565Z","level":"DEBUG","fields":{"message":"enter"},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"test","name":"system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.074771Z","level":"DEBUG","fields":{"message":"some debug info","number":42},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"test","name":"system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.074911Z","level":"DEBUG","fields":{"message":"exit"},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.075098Z","level":"DEBUG","fields":{"message":"enter"},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"other test","name":"other_system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.075291Z","level":"DEBUG","fields":{"message":"some other debug info","digits":17},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"other test","name":"other_system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.075428Z","level":"DEBUG","fields":{"message":"exit"},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.075745Z","level":"INFO","fields":{"message":"Starting step 0 at simulation time 0.00000 (dt = 1.66667e-2)"},"target":"dynamecs_app","span":{"step_index":0,"name":"step"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.075945Z","level":"DEBUG","fields":{"message":"enter"},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"test","name":"system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.076217Z","level":"DEBUG","fields":{"message":"some debug info","number":42},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"test","name":"system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.076413Z","level":"DEBUG","fields":{"message":"exit"},"target":"basic_app","span":{"something":"test","name":"system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.076594Z","level":"DEBUG","fields":{"message":"enter"},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"other test","name":"other_system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.076780Z","level":"DEBUG","fields":{"message":"some other debug info","digits":17},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"},{"something":"other test","name":"other_system_that_logs"}]}
{"timestamp":"2023-03-29T18:16:50.076916Z","level":"DEBUG","fields":{"message":"exit"},"target":"basic_app","span":{"something":"other test","name":"other_system_that_logs"},"spans":[{"name":"run"},{"step_index":0,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.077044Z","level":"INFO","fields":{"message":"exit"},"target":"dynamecs_app","span":{"step_index":0,"name":"step"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.077210Z","level":"INFO","fields":{"message":"enter"},"target":"dynamecs_app","span":{"step_index":1,"name":"step"},"spans":[{"name":"run"},{"step_index":1,"name":"step"}]}
{"timestamp":"2023-03-29T18:16:50.077323Z","level":"INFO","fields":{"message":"exit"},"target":"dynamecs_app","span":{"step_index":1,"name":"step"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.077568Z","level":"INFO","fields":{"message":"Simulation ended"},"target":"dynamecs_app","span":{"name":"run"},"spans":[{"name":"run"}]}
{"timestamp":"2023-03-29T18:16:50.077681Z","level":"INFO","fields":{"message":"exit"},"target":"dynamecs_app","span":{"name":"run"},"spans":[]}
    "###;
    let records: Vec<_> = iterate_records_from_reader(log_data.as_bytes())
        .collect::<eyre::Result<_>>()
        .unwrap();
    let timings = accumulate_timings(&records).unwrap();
    insta::assert_debug_snapshot!(timings);
}