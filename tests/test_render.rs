use assert_cmd::prelude::*;
use predicates::prelude::*;
use pretty_assertions::assert_eq;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

const BIN_NAME: &'static str = "helm-templexer";

#[test]
fn render_config_example() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    cmd.current_dir("tests/data")
        .arg("render")
        .arg("config_example.toml");

    cmd.assert().success();

    // assert that all the deployment directories exist
    assert_eq!(PathBuf::from("tests/data/manifests").exists(), true);
    assert_eq!(
        PathBuf::from("tests/data/manifests/edge-eu-w4").exists(),
        true
    );
    assert_eq!(
        PathBuf::from("tests/data/manifests/stage-eu-w4").exists(),
        true
    );
    assert_eq!(
        PathBuf::from("tests/data/manifests/prod-eu-w4").exists(),
        true
    );
    assert_eq!(
        PathBuf::from("tests/data/manifests/next-edge-eu-w4").exists(),
        false
    );

    // asert that the release name override for prod-eu-e4 worked
    assert_eq!(
        PathBuf::from("tests/data/manifests/prod-eu-w4/my-app-prod-eu-w4").exists(),
        true
    );

    // dig deep into some of the rendered manifest files
    assert_eq!(
        PathBuf::from("tests/data/manifests/edge-eu-w4/my-app/nginx-chart/templates").exists(),
        true
    );

    let mut edge_deployment_yaml = std::fs::File::open(
        "tests/data/manifests/edge-eu-w4/my-app/nginx-chart/templates/deployment.yaml",
    )?;
    let mut contents = "".to_string();
    edge_deployment_yaml.read_to_string(&mut contents)?;
    assert_eq!(contents.contains("image: \"nginx:latest\""), true);

    // todo extend assertions here while changing the chart under test
    // todo this test could also benefit from some utility functions/macros to make it less verbose

    // clean up file wrote to disk
    std::fs::remove_dir_all("tests/data/manifests")?;

    Ok(())
}

#[test]
fn render_config_example_to_stdout() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    cmd.current_dir("tests/data")
        .arg("render")
        .arg("--stdout")
        .arg("config_example.toml");

    // we cannot simply match the stdout data against a pre-rendered file because helm
    // can change the order of resources - hence a test would be flaky.
    // therefore, we'll use samples to assert the likely correctness of the output
    //
    // So this is intentionally NOT used:
    // let fixture_contents = std::fs::read_to_string("tests/data/manifest.yaml")?;
    // cmd.assert().success().stdout(fixture_contents);

    let deployment_edge = "---
# Source: nginx-chart/templates/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-edge";

    let deployment_stage = "---
# Source: nginx-chart/templates/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-stage";

    let deployment_prod = "---
# Source: nginx-chart/templates/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-prod-eu-w4";

    let samples = predicate::str::starts_with("---")
        .and(predicate::str::contains(deployment_edge))
        .and(predicate::str::contains(deployment_stage))
        .and(predicate::str::contains(deployment_prod));
    cmd.assert().success().stdout(samples);

    Ok(())
}
