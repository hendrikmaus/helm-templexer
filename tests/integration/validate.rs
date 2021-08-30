use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn file_is_valid() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("helm-templexer")?;

    cmd.current_dir("tests/data")
        .arg("validate")
        .arg("config_example.yaml");

    cmd.assert().success();

    Ok(())
}

#[test]
fn file_does_not_exist() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("helm-templexer")?;

    cmd.arg("validate").arg("this-file-does-not-exist");
    cmd.assert().failure().stderr(predicate::str::contains(
        r#"File "this-file-does-not-exist" does not exist or is not readable"#,
    ));

    Ok(())
}

#[test]
fn chart_does_not_exist() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("helm-templexer")?;

    cmd.current_dir("tests/data")
        .arg("validate")
        .arg("config_chart_does_not_exist.yaml");
    cmd.assert().failure().stderr(predicate::str::contains(
        r#"does not exist or is not readable"#,
    ));

    Ok(())
}

#[test]
fn validate_accepts_multiple_files() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("helm-templexer")?;

    cmd.current_dir("tests/data")
        .arg("validate")
        .arg("config_example.yaml")
        .arg("config_example.yaml")
        .arg("config_example.yaml");
    cmd.assert().success();

    Ok(())
}
