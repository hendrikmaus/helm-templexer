use anyhow::{anyhow, bail};
use log::info;
use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    /// Schema version to use
    pub version: String,

    /// (not implemented) Optional Helm SemVer version constraint to validate against
    pub helm_version: Option<String>,

    /// Activate/deactivate rendering of contained deployments
    pub enabled: Option<bool>,

    /// Chart to use
    pub chart: PathBuf,

    /// Namespace to pass via `--namespace`
    pub namespace: Option<String>,

    /// Release name passed to `helm template` call
    pub release_name: String,

    /// Output path to write manifests to
    pub output_path: PathBuf,

    /// Use any other option that `helm template` supports
    /// Contents are not validated against actual `helm` options
    pub additional_options: Option<Vec<String>>,

    /// Value files to pass via `--values`
    pub values: Option<Vec<PathBuf>>,

    /// List of deployments to render given Chart
    pub deployments: Vec<Deployment>,
}

#[derive(Deserialize, Debug)]
pub struct Deployment {
    /// Name of the deployment, used to create the output path
    pub name: String,

    /// Activate/deactivate rendering of this specific deployment
    pub enabled: Option<bool>,

    /// Override the release name passed to `helm template`
    pub release_name: Option<String>,

    /// Append any additional options to the top level options
    pub additional_options: Option<Vec<String>>,

    /// Append value files to the top level value files
    pub values: Option<Vec<PathBuf>>,
}

#[derive(Default)]
pub struct ValidationOpts {
    pub skip_disabled: bool,
    pub config_file: Option<PathBuf>,
}

impl Config {
    /// Load given configuration file and deserialize it.
    /// Does not call Config::validate - only checks the path and runs Serde
    pub fn load<S: AsRef<Path>>(file: S) -> anyhow::Result<Config> {
        Self::check_file_exists_and_readable(file.as_ref())?;

        let cfg = std::fs::read_to_string(&file)?;
        let cfg = serde_yaml::from_str::<Config>(&cfg)
            .map_err(|err| format_serde_error::SerdeError::new(cfg.clone(), err))?;
        Ok(cfg)
    }

    /// Validate the loaded configuration file
    pub fn validate(&self, opts: &ValidationOpts) -> anyhow::Result<()> {
        if let Some(enabled) = self.enabled {
            if !enabled && opts.skip_disabled {
                info!("Skipped validation of disabled file");
                return Ok(());
            }
        }

        if let Some(config_file) = &opts.config_file {
            // change the working directory to the place where the config file is, so that all
            // paths are relative to the config file instead of the location where the templexer is called from
            let base_path = config_file.parent().ok_or_else(|| {
                anyhow!(
                    "could not determine base path of given configuration file {:?}",
                    config_file
                )
            })?;

            // if we're already next to the config file, the base path will be empty
            if base_path.components().next().is_some() {
                log::trace!("changing base path for execution to {:?}", base_path);
                std::env::set_current_dir(base_path)?;
            }
        }

        self.check_chart_exists_and_readable()?;
        self.check_value_files_exist_and_readable()?;
        self.check_schema_version()?;
        self.check_if_at_least_one_deployment_is_enabled()?;

        Ok(())
    }

    /// Check whether the given input file exists and is readable
    fn check_file_exists_and_readable(input_file: &Path) -> anyhow::Result<()> {
        if !input_file.exists() {
            bail!("File {:?} does not exist or is not readable", input_file);
        }

        Ok(())
    }

    /// Assert that the designated Helm chart can be found on disk
    fn check_chart_exists_and_readable(&self) -> anyhow::Result<()> {
        if !self.chart.exists() {
            bail!("Chart {:?} does not exist or is not readable", self.chart);
        }

        Ok(())
    }

    /// Find all referenced value files in the given config and check if they exist
    fn check_value_files_exist_and_readable(&self) -> anyhow::Result<()> {
        match &self.values {
            Some(values) => Self::check_pathbuf_vec(values)?,
            None => (),
        }

        for deployment in &self.deployments {
            if matches!(deployment.enabled, Some(enabled) if !enabled) {
                continue;
            }

            match &deployment.values {
                Some(values) => Self::check_pathbuf_vec(values)?,
                None => (),
            }
        }

        Ok(())
    }

    /// Helper to iterate a vector of paths and check their existence
    fn check_pathbuf_vec(files: &[PathBuf]) -> anyhow::Result<()> {
        for f in files {
            if !f.exists() {
                bail!("values file {:?} does not exist or is not readable", f)
            }
        }

        Ok(())
    }

    /// Check the given schema version; should be extended once multiple are available
    fn check_schema_version(&self) -> anyhow::Result<()> {
        if self.version != "v2" {
            bail!("invalid schema version used; only 'v2' is supported")
        }

        Ok(())
    }

    /// Go through all deployments and check if at least one of them is enabled
    fn check_if_at_least_one_deployment_is_enabled(&self) -> anyhow::Result<()> {
        let mut all_disabled = true;

        for d in &self.deployments {
            match d.enabled {
                Some(e) => {
                    if e {
                        all_disabled = false;
                        break;
                    }
                }
                None => {
                    all_disabled = false;
                    break;
                }
            }
        }

        if all_disabled {
            bail!("All deployments are disabled")
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO these are duplicated in render_cmd as well
    //      time to create a unit test module?
    fn get_config() -> Config {
        Config {
            version: "v2".to_string(),
            helm_version: None,
            enabled: Some(true),
            chart: Default::default(),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![],
        }
    }

    fn get_deployment() -> Deployment {
        Deployment {
            name: "".to_string(),
            enabled: Some(true),
            release_name: None,
            additional_options: None,
            values: None,
        }
    }

    #[test]
    #[should_panic]
    fn input_file_does_not_exist() {
        // TODO feels more like an integration test, rather than a unit test
        Config::load("does-not-exist").unwrap();
    }

    #[test]
    fn input_file_exists() {
        // TODO feels more like an integration test, rather than a unit test
        Config::load("tests/data/config_example.yaml").unwrap();
    }

    #[test]
    #[should_panic]
    fn schema_version_must_be_v1() {
        let mut cfg = get_config();
        cfg.version = "invalid".to_string();
        cfg.enabled = Some(false);

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        cfg.deployments = vec![deployment];

        cfg.check_schema_version().unwrap();
    }

    #[test]
    fn disabled_files_can_be_skipped_during_validation() {
        let mut cfg = get_config();
        cfg.version = "invalid".to_string();
        cfg.enabled = Some(false);

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        cfg.deployments = vec![deployment];

        cfg.validate(&ValidationOpts {
            skip_disabled: true,
            config_file: Default::default(),
        })
        .unwrap();
    }

    #[test]
    fn disabled_deployments_can_be_skipped_during_validation() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("tests/data/nginx-chart");

        let mut edge_deployment = get_deployment();
        edge_deployment.name = "edge".to_string();

        let mut stage_deployment = get_deployment();
        stage_deployment.name = " stage".to_string();
        stage_deployment.enabled = Some(false);
        stage_deployment.values = Some(vec![PathBuf::from("does-not-exist")]);

        cfg.deployments = vec![edge_deployment, stage_deployment];

        cfg.validate(&ValidationOpts {
            skip_disabled: true,
            config_file: Default::default(),
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_if_all_deployments_are_disabled() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("tests/data/nginx-chart");

        let mut edge_deployment = get_deployment();
        edge_deployment.name = "edge".to_string();
        edge_deployment.enabled = Some(false);

        let mut stage_deployment = get_deployment();
        stage_deployment.name = " stage".to_string();
        stage_deployment.enabled = Some(false);

        cfg.deployments = vec![edge_deployment, stage_deployment];

        cfg.validate(&ValidationOpts::default()).unwrap();
    }
}
