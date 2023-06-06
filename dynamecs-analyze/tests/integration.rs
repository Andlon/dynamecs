use escargot::CargoBuild;
use insta::assert_snapshot;
use tempfile::tempdir;
use time::{Date, Month, UtcOffset};
use dynamecs_analyze::{iterate_records, iterate_records_from_reader, Record, RecordBuilder, write_records};

fn replace_middle(s: &str, prefix: &str, suffix: &str, replacement: &str) -> Option<String> {
    s.strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
        .map(|_| format!("{prefix}{replacement}{suffix}"))
}

fn redact_message(msg: &str) -> String {
    // Replace the "middle" part of the pattern "<prefix><middle><suffix>"
    let path_replacement = "<redacted path>";
    let path_redactions = [
        // prefix, suffix
        ["Working directory: ", ""],
        ["Logging text to file ", " with log level trace"],
        ["Logging JSON to file ", " with log level trace"],
        ["Archived log file path: ", ""],
        ["Archived JSON log file path: ", ""],
        ["Output base path: ", ""],
    ];
    for [prefix, suffix] in path_redactions {
        if let Some(replaced) = replace_middle(msg, prefix, suffix, path_replacement) {
            return replaced;
        }
    }

    msg.to_string()
}

fn redact_records(records: &[Record]) -> Vec<Record> {
    let arbitrary_timestamp = Date::from_calendar_date(2000, Month::November, 14)
        .unwrap()
        .with_hms(08, 00, 00)
        .unwrap()
        .assume_offset(UtcOffset::from_hms(02, 00, 00).unwrap());
    records.iter()
        .cloned()
        .map(|record| {
            let message_override = record.message()
                .map(|message| redact_message(message));

            let mut builder = RecordBuilder::from_record(record)
                .timestamp(arbitrary_timestamp)
                .thread_id("ThreadId(0)");

            if let Some(msg) = message_override {
                builder = builder.message(msg);
            }

            builder.build()
        })
        .collect()
}

#[test]
fn test_expected_output_for_basic_app1() -> Result<(), Box<dyn std::error::Error>> {
    // We test that the JSON records that we manually write with write_records
    // are equivalent when imported again to the expected output from an artificial,
    // minimal dynamecs-app application
    let temp_dir = tempdir()?;
    let target_dir = temp_dir.path().join("target");
    let output_dir = temp_dir.path().join("output");
    let _ = CargoBuild::new()
        .manifest_path("../test-apps/Cargo.toml")
        .bin("basic_app1")
        .target_dir(target_dir)
        .run()?
        .command()
        .args(["--output-dir", output_dir.to_str().unwrap()])
        .args(["--file-log-level", "trace"])
        .output()?;

    let records: Vec<_> = iterate_records(output_dir.join("logs/dynamecs_app.jsonlog"))?
        .map(|record_result| record_result.expect("The records are complete/correct for this test"))
        .collect();
    let redacted_records = redact_records(&records);

    let mut bytes: Vec<u8> = vec![];
    write_records(&mut bytes, redacted_records.clone().into_iter())?;
    let records_string = String::from_utf8(bytes.clone())?;

    assert_snapshot!(records_string);

    let records_roundtrip: Vec<_> = iterate_records_from_reader(bytes.as_slice())
        .map(|record_result| record_result.expect("Records must be complete"))
        .collect();

    assert_eq!(records_roundtrip, redacted_records);

    Ok(())
}