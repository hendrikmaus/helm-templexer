use assert_cmd::prelude::*;
use cmd_lib::run_fun;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const BIN_NAME: &'static str = env!("CARGO_PKG_NAME");

fn generate_config() -> Result<PathBuf, io::Error> {
    // Generate the Config in string literal
    let config = r#"---
version: v2
enabled: true
chart: ../nginx-chart
namespace: my-namespace
release_name: my-app
output_path: manifests
additional_options:
  - "--skip-crds"
  - "--no-hooks"
values:
  - ../nginx-chart/values/default.yaml
deployments:
  - name: edge-eu-w4
    values:
      - ../nginx-chart/values/edge.yaml
    additional_options:
      - "--set image.tag=latest"
  - name: next-edge-eu-w4
    enabled: false
    values:
      - ../nginx-chart/values/edge.yaml
      - ../nginx-chart/values/next-edge.yaml
  - name: stage-eu-w4
    values:
      - ../nginx-chart/values/stage.yaml
  - name: prod-eu-w4
    release_name: my-app-prod-eu-w4
    values:
      - ../nginx-chart/values/prod.yaml
      - ../nginx-chart/values/prod-eu-w4.yaml
"#;

    // Create a temp folder
    let config_folder = run_fun!(mktemp -d "tests/data"/configXXXX)?;

    let path = format!("{}/config.yaml", config_folder);

    // Create a config file and put it in the config folder
    let mut tmp_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(&path)
        .unwrap();

    // Write the generated string literal to a file in the temp folder
    writeln!(tmp_file, "{}", config)?;

    // return the entire folder
    Ok(PathBuf::from(path))
}

fn drop_temp_folders(dir: &str) -> Result<String, io::Error> {
    // delete the tmp folder
    Ok(run_fun!(rm -r $dir).unwrap())
}

#[test]
fn render_config_example() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = generate_config()?;

    cmd.arg("render").arg(&config);

    cmd.assert().success();

    // manifests parent folder
    let manifests_folder = format!("{}/manifests", config.parent().unwrap().to_string_lossy());

    // edge manifests folder
    let edge_manifests_folder = format!(
        "{}/manifests/edge-eu-w4",
        config.parent().unwrap().to_string_lossy()
    );

    let stage_manifest_folder = format!(
        "{}/manifests/stage-eu-w4",
        config.parent().unwrap().to_string_lossy()
    );

    let prod_manifest_folder = format!(
        "{}/manifests/prod-eu-w4",
        config.parent().unwrap().to_string_lossy()
    );

    let next_edge_manifest_folder = format!(
        "{}/manifests/next-edge-eu-w4",
        config.parent().unwrap().to_string_lossy()
    );

    // assert that all the deployment directories exist
    assert_eq!(PathBuf::from(&manifests_folder).exists(), true);
    assert_eq!(PathBuf::from(&edge_manifests_folder).exists(), true);
    assert_eq!(PathBuf::from(stage_manifest_folder).exists(), true);
    assert_eq!(PathBuf::from(prod_manifest_folder).exists(), true);
    assert_eq!(PathBuf::from(next_edge_manifest_folder).exists(), false);

    // // asert that the release name override for prod-eu-e4 worked
    assert_eq!(
        PathBuf::from(format!(
            "{}/prod-eu-w4/my-app-prod-eu-w4/manifest.yaml",
            manifests_folder
        ))
        .exists(),
        true
    );

    assert_eq!(
        PathBuf::from(format!(
            "{}/edge-eu-w4/my-app/manifest.yaml",
            manifests_folder
        ))
        .exists(),
        true
    );

    let edge_rendered_output = format!(
        "{}/manifests/edge-eu-w4/my-app/manifest.yaml",
        config.parent().unwrap().to_string_lossy()
    );

    let mut edge_deployment_yaml = std::fs::File::open(edge_rendered_output).unwrap();
    let mut contents = "".to_string();
    edge_deployment_yaml.read_to_string(&mut contents)?;
    assert_eq!(contents.contains("image: \"nginx:latest\""), true);

    assert_eq!(
        contents,
        include_str!("../../tests/data/rendered_manifests/edge-eu-w4/my-app/manifest.yaml")
    );

    // // todo extend assertions here while changing the chart under test
    // // todo this test could also benefit from some utility functions/macros to make it less verbose

    // delete temp folder
    drop_temp_folders(&format!("{}", config.parent().unwrap().to_string_lossy()))?;

    Ok(())
}

#[test]
fn pipe_output_to_a_tool_that_exists() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = generate_config()?;

    cmd.arg("render").arg("--pipe=grep 'image'").arg(&config);

    cmd.assert().success();

    let edge_rendered_output = format!(
        "{}/manifests/edge-eu-w4/my-app/manifest.yaml",
        config.parent().unwrap().to_string_lossy()
    );

    let mut yaml = std::fs::File::open(edge_rendered_output).unwrap();

    let mut contents = "".to_string();
    yaml.read_to_string(&mut contents)?;

    // assert that the output contains only images related content
    assert_eq!(
        contents.contains(
            "          image: \"nginx:latest\"\n          imagePullPolicy: IfNotPresent\n"
        ),
        true
    );

    // delete temp folder
    drop_temp_folders(&format!("{}", config.parent().unwrap().to_string_lossy()))?;

    Ok(())
}

#[test]
fn pipe_output_to_multiple_tools() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = generate_config().unwrap();

    cmd.arg("render")
        .arg("--pipe=grep 'image'")
        .arg("--pipe=grep 'imagePullPolicy'")
        .arg(&config);

    cmd.assert().success();

    let edge_rendered_output = format!(
        "{}/manifests/edge-eu-w4/my-app/manifest.yaml",
        config.parent().unwrap().to_string_lossy()
    );

    let mut yaml = std::fs::File::open(edge_rendered_output).unwrap();

    let mut contents = "".to_string();
    yaml.read_to_string(&mut contents)?;

    // assert that the output contains only imagePullPolicy related content
    assert_eq!(
        contents.contains("          imagePullPolicy: IfNotPresent\n"),
        true
    );

    // delete temp folder
    drop_temp_folders(&format!("{}", config.parent().unwrap().to_string_lossy()))?;

    Ok(())
}

#[test]
fn pipe_output_to_a_tool_that_doesnt_exist() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = generate_config().unwrap();

    cmd.arg("render").arg("--pipe=xyz 'image'").arg(&config);

    // the binary execution should fail because xyz tool doesn't exist.
    cmd.assert().failure();

    // delete temp folder
    drop_temp_folders(&format!("{}", config.parent().unwrap().to_string_lossy()))?;

    Ok(())
}
