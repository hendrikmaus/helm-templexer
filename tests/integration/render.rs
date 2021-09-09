use assert_cmd::prelude::*;
use cmd_lib::run_fun;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const BIN_NAME: &'static str = env!("CARGO_PKG_NAME");

struct Config {
    temp_dir: PathBuf,
    path: PathBuf,
}

impl Config {
    /// Create a new config in a unique location; can be `drop`'ed after usage
    fn new() -> anyhow::Result<Self> {
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

        let config_folder = run_fun!(mktemp -d "tests/data"/test_config_XXXX)?;
        let path = format!("{}/config.yaml", &config_folder);
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&path)
            .unwrap();

        writeln!(tmp_file, "{}", config)?;

        Ok(Config {
            temp_dir: PathBuf::from(config_folder.to_owned()),
            path: PathBuf::from(path.to_owned()),
        })
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.temp_dir)
            .expect("Failed to drop temporary directory of config")
    }
}

#[test]
fn render_config_example() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = Config::new()?;

    cmd.arg("render").arg(&config.path);

    cmd.assert().success();

    // manifests parent folder
    let manifests_folder = format!("{}/manifests", config.temp_dir.to_string_lossy());

    // edge manifests folder
    let edge_manifests_folder =
        format!("{}/manifests/edge-eu-w4", config.temp_dir.to_string_lossy());

    let stage_manifest_folder = format!(
        "{}/manifests/stage-eu-w4",
        config.temp_dir.to_string_lossy()
    );

    let prod_manifest_folder =
        format!("{}/manifests/prod-eu-w4", config.temp_dir.to_string_lossy());

    let next_edge_manifest_folder = format!(
        "{}/manifests/next-edge-eu-w4",
        config.temp_dir.to_string_lossy()
    );

    // assert that all the deployment directories exist
    assert_eq!(PathBuf::from(&manifests_folder).exists(), true);
    assert_eq!(PathBuf::from(&edge_manifests_folder).exists(), true);
    assert_eq!(PathBuf::from(stage_manifest_folder).exists(), true);
    assert_eq!(PathBuf::from(prod_manifest_folder).exists(), true);
    assert_eq!(PathBuf::from(next_edge_manifest_folder).exists(), false);

    // asert that the release name override for prod-eu-e4 worked
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
        config.temp_dir.to_string_lossy()
    );

    let mut edge_deployment_yaml = std::fs::File::open(edge_rendered_output)?;
    let mut contents = "".to_string();
    edge_deployment_yaml.read_to_string(&mut contents)?;
    assert_eq!(contents.contains("image: \"nginx:latest\""), true);

    assert_eq!(
        contents,
        include_str!("../../tests/data/rendered_manifests/edge-eu-w4/my-app/manifest.yaml")
    );

    Ok(())
}

#[test]
fn pipe_output_to_a_tool_that_exists() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = Config::new()?;

    cmd.arg("render")
        .arg("--pipe=grep 'image'")
        .arg(&config.path);

    cmd.assert().success();

    let edge_rendered_output = format!(
        "{}/manifests/edge-eu-w4/my-app/manifest.yaml",
        config.temp_dir.to_string_lossy()
    );

    let mut yaml = std::fs::File::open(edge_rendered_output)?;

    let mut contents = "".to_string();
    yaml.read_to_string(&mut contents)?;

    // assert that the output contains only images related content
    assert_eq!(
        contents.trim_start(),
        "image: \"nginx:latest\"\n          imagePullPolicy: IfNotPresent\n"
    );

    Ok(())
}

#[test]
fn pipe_output_to_multiple_tools() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;

    let config = Config::new()?;

    cmd.arg("render")
        .arg("--pipe=grep 'image'")
        .arg("--pipe=grep 'imagePullPolicy'")
        .arg(&config.path);

    cmd.assert().success();

    let edge_rendered_output = format!(
        "{}/manifests/edge-eu-w4/my-app/manifest.yaml",
        config.temp_dir.to_string_lossy()
    );

    let mut yaml = std::fs::File::open(edge_rendered_output)?;

    let mut contents = "".to_string();
    yaml.read_to_string(&mut contents)?;

    // assert that the output contains only imagePullPolicy related content
    assert_eq!(contents.trim_start(), "imagePullPolicy: IfNotPresent\n");

    Ok(())
}

#[test]
fn pipe_output_to_a_tool_that_doesnt_exist() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    let config = Config::new()?;

    cmd.arg("render")
        .arg("--pipe=xyz 'image'")
        .arg(&config.path);

    // the binary execution should fail because xyz tool doesn't exist.
    cmd.assert().failure();

    Ok(())
}

#[test]
fn render_multiple_files() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    let config0 = Config::new()?;
    let config1 = Config::new()?;
    cmd.arg("render").arg(&config0.path).arg(&config1.path);
    cmd.assert().success();

    Ok(())
}
