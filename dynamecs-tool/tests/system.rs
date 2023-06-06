use std::error::Error;
use escargot::CargoBuild;
use tempfile::tempdir;

#[test]
fn test_timing_basic_app1_all_steps() -> Result<(), Box<dyn Error>> {
    // We use a fixed logfile to avoid the problem of having to account for
    // varying durations. To re-generate a log file, run from the project root:
    //  $ cd test_apps
    //  $ cargo run --bin basic_app1
    //  $ cp output/logs/dynamecs_log.jsonlog ../dynamecs-tool/tests/test_logs/

    let temp_dir = tempdir()?;
    let target_dir = temp_dir.path().join("target");

    // Then run dynamecs-tool
    let output = CargoBuild::new()
        .bin("dynamecs-tool")
        .target_dir(target_dir)
        .run()?
        .command()
        .arg("timing")
        .args(["--logfile", "tests/test_logs/dynamecs_app.jsonlog"])
        .output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout_str = String::from_utf8(output.stdout)?;
    insta::assert_snapshot!(stdout_str);

    Ok(())
}