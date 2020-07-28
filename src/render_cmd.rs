use crate::config::{Config, ValidationOpts};
use crate::RenderCmdOpts;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use subprocess::{Exec, Redirection};

pub struct RenderCmd {
    opts: RenderCmdOpts,
}

/// Plan which contains all commands to be executed
/// Can be skipped if the config is disabled at the top level
/// Disabled deployments are not added to the plan
struct Plan {
    /// Skip this plan; set to true if the config is disabled on the top level
    skip: bool,

    /// Commands to be executed on the host system
    /// key: deployment.name
    /// value: vector of strings containing the complete command, e.g. vec!["helm", "template", ...]
    commands: HashMap<String, Vec<String>>,

    /// Fully qualified output path that will be passed to `helm template`
    /// the output path is built as follows:
    ///   <config.output_path>/<deployment.name>/<[config|deployment].release_name>
    ///
    /// key: deployment.name
    output_paths: HashMap<String, PathBuf>,
}

impl RenderCmd {
    /// Create sub command struct to render deployments of the given input file(s)
    pub fn new(opts: RenderCmdOpts) -> Self {
        Self { opts }
    }

    /// Main entry point to run the rendering process
    /// will return nothing on the happy path and descriptive errors on failure
    pub fn run(&self) -> Result<()> {
        debug!("render options: {:?}", self.opts);

        for file in &self.opts.input_files {
            info!("rendering deployments for {:?}", file);

            let cfg = Config::load(&file)?;
            cfg.validate(&ValidationOpts::default())?;

            let plan = self.plan(&cfg)?;

            if plan.skip {
                warn!("config is disabled (skipped)");
                continue;
            }

            self.exec_plan(&plan)?;
        }

        info!("done");

        Ok(())
    }

    /// Create a plan of commands to execute
    fn plan(&self, cfg: &Config) -> Result<Plan> {
        let mut plan = Plan {
            skip: false,
            commands: Default::default(),
            output_paths: Default::default(),
        };

        if let Some(enabled) = cfg.enabled {
            if !enabled {
                plan.skip = true;
                return Ok(plan);
            }
        }

        // todo is it really that hard to get from PathBuf to String?
        let chart = match cfg.chart.to_str() {
            Some(s) => s,
            None => bail!(
                "failed to convert given chart path {:?} to string",
                cfg.chart
            ),
        };

        let output_dir = match cfg.output_path.to_str() {
            Some(s) => s,
            None => bail!(
                "failed to convert given output_path {:?} to string",
                cfg.output_path
            ),
        };

        let values: Vec<String> = self
            .get_values_as_strings(&cfg.values)?
            .iter()
            .map(|f| format!("--values={}", f))
            .collect();

        let mut base_cmd = vec![];
        base_cmd.push("helm".to_string());
        base_cmd.push("template".to_string());
        base_cmd.push(cfg.release_name.clone());
        base_cmd.push(chart.to_string());

        match &cfg.namespace {
            Some(namespace) => base_cmd.push(format!("--namespace={}", namespace)),
            None => (),
        }

        base_cmd.extend(values);

        match &cfg.additional_options {
            Some(opts) => base_cmd.extend(opts.clone()),
            None => (),
        }

        for d in &cfg.deployments {
            if let Some(enabled) = d.enabled {
                if !enabled {
                    warn!(" - {} (skipped)", d.name);
                    continue;
                }
            }

            let mut cmd = base_cmd.clone();

            let values: Vec<String> = self
                .get_values_as_strings(&d.values)?
                .iter()
                .map(|f| format!("--values={}", f))
                .collect();

            cmd.extend(values);

            match &d.additional_options {
                Some(opts) => cmd.extend(opts.clone()),
                None => (),
            }

            let mut release_name = cfg.release_name.clone();
            match &d.release_name {
                Some(n) => release_name = n.to_owned(),
                None => (),
            }
            cmd[2] = release_name.to_owned();

            let fully_qualified_output_dir = format!("{}/{}/{}", output_dir, d.name, release_name);
            cmd.push(format!("--output-dir={}", fully_qualified_output_dir));

            plan.commands.insert(d.name.to_owned(), cmd);
            plan.output_paths
                .insert(d.name.clone(), PathBuf::from(fully_qualified_output_dir));
        }

        Ok(plan)
    }

    /// Execute the commands in the given plan
    fn exec_plan(&self, plan: &Plan) -> Result<()> {
        for (deployment, cmd) in &plan.commands {
            info!(" - {}", deployment);

            debug!(
                "executing planned command for deployment {}:\n \t {:#?}",
                deployment,
                cmd.join(" ")
            );

            // todo allow the user to disable cleanup of the output path?
            match &plan.output_paths.get(deployment) {
                Some(p) => {
                    debug!("cleaning up output path: {:?}", p);
                    if p.exists() {
                        std::fs::remove_dir_all(p)?;
                    }
                    std::fs::create_dir_all(p)?;
                }
                None => (),
            }

            // `helm` logs that it wanted to exit 1 but actually exits 0:
            //
            //   ❯ helm version --client
            //   version.BuildInfo{Version:"v3.2.4", GitCommit:"0ad800ef43d3b826f31a5ad8dfbb4fe05d143688", GitTreeState:"clean", GoVersion:"go1.13.12"}
            //
            //   ❯ helm faulty-command
            //   Error: unknown command "faulty-command" for "helm"
            //   Run 'helm --help' for usage.
            //       exit status 1
            //
            //   ❯ echo $?
            //   0
            //
            // So we'll examine stdout/stderr to detect if helm failed but did not exit
            // with a code other than 0.
            //
            // The issue is reported and open https://github.com/helm/helm/issues/8268
            //   as of 2020-07-26

            let result = Exec::shell(cmd.join(" "))
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge)
                .capture()?;

            debug!("helm output:\n{}", result.stdout_str());

            if !result.exit_status.success() || result.stdout_str().contains("exit status 1") {
                bail!(
                    "failed while running `helm`:\n\n\t{}\n\n{}",
                    cmd.join(" "),
                    result.stdout_str()
                );
            }
        }

        Ok(())
    }

    /// Utility to turn an option for a vector of pathbufs into a vector of strings
    /// todo is it really that hard to go from a Vec<PathBuf> to a Vec<String>?
    fn get_values_as_strings(&self, input: &Option<Vec<PathBuf>>) -> Result<Vec<String>> {
        let mut buffer: Vec<String> = vec![];

        match input {
            Some(items) => {
                for i in items {
                    match i.clone().into_os_string().into_string() {
                        Ok(s) => buffer.push(s),
                        Err(s) => bail!("failed to convert {:?} to string", s),
                    }
                }
            }
            None => (),
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Deployment;
    use pretty_assertions::assert_eq;

    #[test]
    fn disabled_files_are_skipped() {
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
            deployments: vec![],
        };

        let cmd = RenderCmd {
            opts: RenderCmdOpts {
                input_files: vec![],
                helm_bin: None,
            },
        };

        let res = cmd.plan(&cfg).unwrap();
        assert_eq!(true, res.skip);
    }

    #[test]
    fn simple_deployment_command() {
        let cfg = Config {
            version: "v1".to_string(),
            helm_version: None,
            enabled: Option::from(true),
            chart: PathBuf::from("charts/some-chart"),
            namespace: Option::from("default".to_string()),
            release_name: "some-release".to_string(),
            output_path: PathBuf::from("manifests"),
            additional_options: Option::from(vec!["--no-hooks".to_string(), "--debug".to_string()]),
            values: Option::from(vec![PathBuf::from("some-base.yaml")]),
            deployments: vec![Deployment {
                name: "edge".to_string(),
                enabled: Option::from(true),
                release_name: None,
                additional_options: Option::from(vec!["--set=env=edge".to_string()]),
                values: Option::from(vec![PathBuf::from("edge.yaml")]),
            }],
        };

        let cmd = RenderCmd {
            opts: RenderCmdOpts {
                input_files: vec![],
                helm_bin: None,
            },
        };

        let res = cmd.plan(&cfg).unwrap();
        let expected_helm_cmd = "helm template some-release charts/some-chart --namespace=default \
            --values=some-base.yaml --no-hooks --debug --values=edge.yaml --set=env=edge \
            --output-dir=manifests/edge/some-release";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        assert_eq!(&expected_helm_cmd, res.commands.get("edge").unwrap());
    }

    #[test]
    fn disabled_deployments_are_not_planned() {
        let cfg = Config {
            version: "v1".to_string(),
            helm_version: None,
            enabled: Option::from(true),
            chart: PathBuf::from("charts/some-chart"),
            namespace: None,
            release_name: "some-release".to_string(),
            output_path: PathBuf::from("manifests"),
            additional_options: None,
            values: None,
            deployments: vec![Deployment {
                name: "edge".to_string(),
                enabled: Option::from(false),
                release_name: None,
                additional_options: None,
                values: None,
            }],
        };

        let cmd = RenderCmd {
            opts: RenderCmdOpts {
                input_files: vec![],
                helm_bin: None,
            },
        };

        let res = cmd.plan(&cfg).unwrap();
        assert_eq!(None, res.commands.get("edge"));
    }

    #[test]
    fn deployment_can_override_release_name() {
        let cfg = Config {
            version: "v1".to_string(),
            helm_version: None,
            enabled: Option::from(true),
            chart: PathBuf::from("charts/some-chart"),
            namespace: None,
            release_name: "some-release".to_string(),
            output_path: PathBuf::from("manifests"),
            additional_options: None,
            values: None,
            deployments: vec![Deployment {
                name: "edge".to_string(),
                enabled: Option::from(true),
                release_name: Option::from("edge-release".to_string()),
                additional_options: None,
                values: None,
            }],
        };

        let cmd = RenderCmd {
            opts: RenderCmdOpts {
                input_files: vec![],
                helm_bin: None,
            },
        };

        let res = cmd.plan(&cfg).unwrap();
        let expected_helm_cmd = "helm template edge-release charts/some-chart \
            --output-dir=manifests/edge/edge-release";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        assert_eq!(&expected_helm_cmd, res.commands.get("edge").unwrap());
    }
}
