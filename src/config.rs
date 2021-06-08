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

    /// Output path to write manifests to; passed via `--output-dir`
    /// ignored if `--stdout` is passed to the `helm-templexer render`
    /// todo turn it into an `Option` and revert to stdout printing if it is omitted
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
}

impl Config {
    /// Load given configuration file and deserialize it.
    /// Does not call Config::validate - only checks the path and runs Serde
    pub fn load<S: AsRef<Path>>(file: S) -> anyhow::Result<Config> {
        Self::check_file_exists_and_readable(&file.as_ref())?;

        match serde_any::from_file(file) {
            Ok(cfg) => Ok(cfg),
            Err(err) => Err(anyhow!(
                "Failed to load configuration from file, because:\n{}",
                err
            )),
        }
    }

    /// Validate the loaded configuration file
    pub fn validate(&self, opts: &ValidationOpts) -> anyhow::Result<()> {
        if let Some(enabled) = self.enabled {
            if !enabled && opts.skip_disabled {
                info!("Skipped validation of disabled file");
                return Ok(());
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
            Some(values) => Self::check_pathbuf_vec(&values)?,
            None => (),
        }

        for deployment in &self.deployments {
            if matches!(deployment.enabled, Some(enabled) if !enabled) {
                continue;
            }

            match &deployment.values {
                Some(values) => Self::check_pathbuf_vec(&values)?,
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
        if self.version != "v1" {
            bail!("invalid schema version used; only 'v1' is supported")
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

    #[test]
    #[should_panic]
    fn input_file_does_not_exist() {
        // TODO feels more like an integration test, rather than a unit test
        Config::load("does-not-exist").unwrap();
    }

    #[test]
    fn input_file_exists() {
        // TODO feels more like an integration test, rather than a unit test
        Config::load("tests/data/config_example.toml").unwrap();
    }

    #[test]
    #[should_panic]
    fn schema_version_must_be_v1() {
        let cfg = Config {
            version: "v2".to_string(),
            helm_version: None,
            enabled: Option::from(false),
            chart: Default::default(),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![Deployment {
                name: "edge".to_string(),
                enabled: Option::from(true),
                release_name: None,
                additional_options: None,
                values: None,
            }],
        };

        cfg.check_schema_version().unwrap();
    }

    #[test]
    fn disabled_files_can_be_skipped_during_validation() {
        let cfg = Config {
            version: "invalid_version".to_string(),
            helm_version: None,
            enabled: Option::from(false),
            chart: Default::default(),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![Deployment {
                name: "edge".to_string(),
                enabled: Option::from(true),
                release_name: None,
                additional_options: None,
                values: None,
            }],
        };

        cfg.validate(&ValidationOpts {
            skip_disabled: true,
        })
        .unwrap();
    }

    #[test]
    fn disabled_deployments_can_be_skipped_during_validation() {
        let cfg = Config {
            version: "v1".to_string(),
            helm_version: None,
            enabled: Option::from(true),
            chart: PathBuf::from("tests/data/nginx-chart"),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![
                Deployment {
                    name: "edge".to_string(),
                    enabled: Option::from(true),
                    release_name: None,
                    additional_options: None,
                    values: None,
                },
                Deployment {
                    name: "stage".to_string(),
                    enabled: Option::from(false),
                    release_name: None,
                    additional_options: None,
                    values: Option::from(vec![PathBuf::from("does-not-exist")]),
                },
            ],
        };

        cfg.validate(&ValidationOpts {
            skip_disabled: true,
        })
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_if_all_deployments_are_disabled() {
        let cfg = Config {
            version: "v1".to_string(),
            helm_version: None,
            enabled: Option::from(false),
            chart: Default::default(),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![
                Deployment {
                    name: "edge".to_string(),
                    enabled: Option::from(false),
                    release_name: None,
                    additional_options: None,
                    values: None,
                },
                Deployment {
                    name: "stage".to_string(),
                    enabled: Option::from(false),
                    release_name: None,
                    additional_options: None,
                    values: None,
                },
            ],
        };

        cfg.validate(&ValidationOpts::default()).unwrap();
    }
}
