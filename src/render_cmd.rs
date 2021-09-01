use crate::config::{Config, ValidationOpts};
use crate::RenderCmdOpts;
use anyhow::{bail, Context};
use indexmap::map::IndexMap;
use log::{debug, info};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use subprocess::{Exec, Redirection};

/// Special name used in the commands map of a plan when a helm dependency update is requested
const PRE_CMD_DEPENDENCY_UPDATE: &str = "helm-dependency-update";

pub struct RenderCmd {
    opts: RenderCmdOpts,
}

/// Plan which contains all commands to be executed
/// Can be skipped if the config is disabled at the top level
/// Disabled deployments are not added to the plan
struct Plan {
    /// Skip this plan; set to true if the config is disabled on the top level
    skip: bool,

    /// Commands to be executed in order of appearance before running `self.commands`.
    /// This field uses a IndexMap to guarantee order of iteration.
    pre_commands: IndexMap<String, Vec<String>>,

    /// Commands to be executed on the host system
    /// key: deployment.name
    /// value: vector of strings containing the complete command, e.g. vec!["helm", "template", ...]
    commands: HashMap<String, (PathBuf, Vec<String>)>,
}

impl RenderCmd {
    /// Create sub command struct to render deployments of the given input file(s)
    pub fn new(opts: RenderCmdOpts) -> Self {
        Self { opts }
    }

    /// Main entry point to run the rendering process
    /// will return nothing on the happy path and descriptive errors on failure
    pub fn run(&self) -> anyhow::Result<()> {
        debug!("render options: {:?}", self.opts);

        for file in &self.opts.input_files {
            info!("processing {:?}", file);

            let cfg = Config::load(&file)?;
            let opts = ValidationOpts {
                config_file: Some(file.clone()),
                ..Default::default()
            };
            cfg.validate(&opts)?;

            let plan = self.plan(cfg)?;

            if plan.skip {
                info!("config is disabled (skipped)");
                continue;
            }

            self.exec_plan(&plan)?;
        }

        Ok(())
    }

    /// Create a plan of commands to execute
    fn plan(&self, cfg: Config) -> anyhow::Result<Plan> {
        let mut plan = Plan {
            skip: false,
            pre_commands: Default::default(),
            commands: Default::default(),
        };

        if let Some(enabled) = cfg.enabled {
            if !enabled {
                plan.skip = true;
                return Ok(plan);
            }
        }

        let chart = match cfg.chart.to_str() {
            Some(s) => s,
            None => bail!(
                "failed to convert given chart path {:?} to string",
                cfg.chart
            ),
        };

        if self.opts.update_dependencies {
            let cmd = vec![
                "helm".to_string(),
                "dependencies".to_string(),
                "update".to_string(),
                chart.to_string(),
            ];
            plan.pre_commands
                .insert(PRE_CMD_DEPENDENCY_UPDATE.to_string(), cmd);
        }

        let values: Vec<String> = self
            .get_values_as_strings(&cfg.values)?
            .iter()
            .map(|f| format!("--values={}", f))
            .collect();

        let mut base_cmd = vec![
            "helm".to_string(),
            "template".to_string(),
            cfg.release_name.clone(),
            chart.to_string(),
        ];

        if let Some(namespace) = cfg.namespace {
            base_cmd.push(format!("--namespace={}", namespace))
        }

        base_cmd.extend(values);

        if let Some(opts) = cfg.additional_options {
            base_cmd.extend(opts);
        }

        match &self.opts.additional_options {
            Some(opts) => base_cmd.extend(opts.clone()),
            None => (),
        }

        for d in cfg.deployments {
            if self.opts.filter.is_some()
                && !self.is_name_filtered(
                    self.opts
                        .filter
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("failed to extract filter"))?,
                    &d.name,
                )?
            {
                info!(" - (skip) {}", d.name);
                continue;
            }
            if let Some(enabled) = d.enabled {
                if !enabled {
                    info!(" - (skip) {}", d.name);
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

            let mut fully_qualified_output = cfg
                .output_path
                .join(&d.name)
                .join(release_name)
                .join("manifest");
            fully_qualified_output.set_extension("yaml");

            if let Some(opts) = &self.opts.pipe {
                let pipe_command: Vec<String> = opts.iter().map(|p| format!("| {}", p)).collect();
                cmd.extend(pipe_command)
            }

            plan.commands
                .insert(d.name.to_owned(), (fully_qualified_output, cmd));
        }

        Ok(plan)
    }

    /// Execute the commands in the given plan
    fn exec_plan(&self, plan: &Plan) -> anyhow::Result<()> {
        if !&plan.pre_commands.is_empty() {
            info!("pre-commands:");

            for (command_id, cmd) in &plan.pre_commands {
                info!(" - {}", command_id);

                debug!(
                    "executing pre-command {}:\n \t {:#?}",
                    command_id,
                    cmd.join(" ")
                );

                self.run_helm(&cmd.join(" "), std::io::sink())?;
            }
        }

        if !&plan.commands.is_empty() {
            info!("deployments:");

            for (deployment, cmd) in &plan.commands {
                info!(" - {}", deployment);

                debug!(
                    "executing planned command for deployment {}:\n \t {:#?}",
                    deployment,
                    cmd.1.join(" ")
                );

                let output_parent = cmd
                    .0
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("missing parent. this should never happen"))?;

                if output_parent.exists() {
                    debug!("cleaning up output path: {:?}", output_parent);
                    std::fs::remove_dir_all(output_parent)?;
                }
                std::fs::create_dir_all(output_parent)?;

                let output_file =
                    std::fs::File::create(&cmd.0).context("can not create output file")?;
                let output_writer = std::io::BufWriter::new(output_file);

                self.run_helm(&cmd.1.join(" "), output_writer)?;
            }
        }

        Ok(())
    }

    /// Run `helm` commands
    ///
    /// With special result handling as `helm` could exit 0 while logging `exit status 1`.
    /// It is unclear if the issue is actually resolved, see
    /// https://github.com/helm/helm/issues/8268
    fn run_helm(&self, cmd: &str, mut output: impl std::io::Write) -> anyhow::Result<()> {
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
        let result = Exec::shell(cmd)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge)
            .capture()?;

        debug!("helm output:\n{}", result.stdout_str());

        if !result.exit_status.success() || result.stdout_str().contains("exit status 1") {
            bail!(
                "failed while running `helm`:\n\n\t{}\n\n{}",
                cmd,
                result.stdout_str()
            );
        }

        output
            .write_all(result.stdout_str().as_bytes())
            .context("can not write to output")?;

        Ok(())
    }

    /// Utility to turn an option for a vector of pathbufs into a vector of strings
    fn get_values_as_strings(&self, input: &Option<Vec<PathBuf>>) -> anyhow::Result<Vec<String>> {
        let mut buffer: Vec<String> = vec![];

        if let Some(items) = input {
            for i in items {
                match i.to_str() {
                    Some(s) => buffer.push(s.to_string()),
                    None => bail!("failed to convert {:?} to string", i),
                }
            }
        }
        Ok(buffer)
    }

    /// Check if deployment name is filtered
    fn is_name_filtered(&self, regex: &str, name: &str) -> anyhow::Result<bool> {
        Ok(Regex::new(regex)?.is_match(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Deployment;
    use pretty_assertions::assert_eq;

    /// Help function to abstract the construction of `Config` for test cases
    /// This is useful once `Config` changes as only this function needs to be changed, not every test case
    fn get_config() -> Config {
        Config {
            version: "v2".to_string(),
            helm_version: None,
            enabled: Option::from(true),
            chart: Default::default(),
            namespace: None,
            release_name: "".to_string(),
            output_path: Default::default(),
            additional_options: None,
            values: None,
            deployments: vec![],
        }
    }

    /// Help function to abstract the construction of `RenderCmd` for test cases
    /// This is useful once `RenderCmd` changes as only this function needs to be changed, not every test case
    fn get_cmd() -> RenderCmd {
        RenderCmd {
            opts: RenderCmdOpts {
                input_files: vec![],
                additional_options: None,
                update_dependencies: false,
                filter: None,
                pipe: None,
            },
        }
    }

    /// Help function to abstract the construction of `Deployment` for test cases
    /// This is useful once `Deployment` changes as only this function needs to be changed, not every test case
    fn get_deployment() -> Deployment {
        Deployment {
            name: "".to_string(),
            enabled: Option::from(true),
            release_name: None,
            additional_options: None,
            values: None,
        }
    }

    #[test]
    fn disabled_files_are_skipped() {
        let mut cfg = get_config();
        cfg.enabled = Option::from(false);
        let cmd = get_cmd();

        let res = cmd.plan(cfg).unwrap();
        assert_eq!(true, res.skip);
    }

    #[test]
    fn simple_deployment_command() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");
        cfg.additional_options =
            Option::from(vec!["--no-hooks".to_string(), "--debug".to_string()]);
        cfg.values = Option::from(vec![PathBuf::from("some-base.yaml")]);

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        deployment.additional_options = Option::from(vec!["--set=env=edge".to_string()]);
        deployment.values = Option::from(vec![PathBuf::from("edge.yaml")]);
        cfg.deployments = vec![deployment];

        let cmd = get_cmd();
        let res = cmd.plan(cfg).unwrap();
        let expected_helm_cmd = "helm template some-release charts/some-chart --namespace=default \
            --values=some-base.yaml --no-hooks --debug --values=edge.yaml --set=env=edge";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        let got = res.commands.get("edge").unwrap();

        assert_eq!(expected_helm_cmd, got.1)
    }

    #[test]
    fn disabled_deployments_are_not_planned() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        deployment.enabled = Option::from(false);
        cfg.deployments = vec![deployment];

        let cmd = get_cmd();
        let res = cmd.plan(cfg).unwrap();
        assert_eq!(None, res.commands.get("edge"));
    }

    #[test]
    fn deployment_can_override_release_name() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        deployment.release_name = Option::from("edge-release".to_string());
        cfg.deployments = vec![deployment];

        let cmd = get_cmd();
        let res = cmd.plan(cfg).unwrap();
        let expected_helm_cmd = "helm template edge-release charts/some-chart";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        let got = res.commands.get("edge").unwrap();

        assert_eq!(expected_helm_cmd, got.1);
    }

    #[test]
    fn render_can_accept_additional_options_via_cli_option() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");
        cfg.additional_options =
            Option::from(vec!["--no-hooks".to_string(), "--debug".to_string()]);
        cfg.values = Option::from(vec![PathBuf::from("some-base.yaml")]);

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        cfg.deployments = vec![deployment];

        let mut cmd = get_cmd();
        cmd.opts.additional_options =
            Option::from(vec!["--set-string=image.tag=424242a".to_string()]);

        let res = cmd.plan(cfg).unwrap();
        let expected_helm_cmd = "helm template some-release charts/some-chart --namespace=default \
            --values=some-base.yaml --no-hooks --debug --set-string=image.tag=424242a";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        let got = res.commands.get("edge").unwrap();

        assert_eq!(expected_helm_cmd, got.1);
    }

    #[test]
    fn render_can_update_dependencies() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.output_path = PathBuf::from("manifests");

        let mut deployment = get_deployment();
        deployment.name = "edge".to_string();
        cfg.deployments = vec![deployment];

        let mut cmd = get_cmd();
        cmd.opts.update_dependencies = true;

        let res = cmd.plan(cfg).unwrap();
        let expected_helm_cmd = "helm dependencies update charts/some-chart";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();

        assert_eq!(
            &expected_helm_cmd,
            res.pre_commands.get(PRE_CMD_DEPENDENCY_UPDATE).unwrap()
        );
    }

    #[test]
    fn filter_only_edge_deployment() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut edge_eu_w4_deployment = get_deployment();
        let mut stage_eu_w4_deployment = get_deployment();
        let mut prod_as_e1_deployment = get_deployment();
        let mut prod_eu_w4_deployment = get_deployment();
        let mut prod_us_c1_deployment = get_deployment();

        edge_eu_w4_deployment.name = "edge_eu_w4_deployment".to_string();
        stage_eu_w4_deployment.name = "stage_eu_w4_deployment".to_string();
        prod_as_e1_deployment.name = "prod_as_e1_deployment".to_string();
        prod_eu_w4_deployment.name = "prod_eu_w4_deployment".to_string();
        prod_us_c1_deployment.name = "prod_us_c1_deployment".to_string();

        cfg.deployments = vec![
            edge_eu_w4_deployment,
            stage_eu_w4_deployment,
            prod_as_e1_deployment,
            prod_eu_w4_deployment,
            prod_us_c1_deployment,
        ];

        let mut cmd = get_cmd();
        cmd.opts.filter = Option::from("edge".to_string());

        let res = cmd.plan(cfg).unwrap();
        let expected_helm_cmd = "helm template some-release charts/some-chart --namespace=default";
        let expected_helm_cmd: Vec<String> = expected_helm_cmd
            .split_whitespace()
            .map(String::from)
            .collect();
        let got = res.commands.get("edge_eu_w4_deployment").unwrap();

        assert_eq!(expected_helm_cmd, got.1);
        assert_eq!(res.commands.len(), 1);
    }

    #[test]
    fn filter_only_prod_deployment() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut edge_eu_w4_deployment = get_deployment();
        let mut stage_eu_w4_deployment = get_deployment();
        let mut prod_as_e1_deployment = get_deployment();
        let mut prod_eu_w4_deployment = get_deployment();
        let mut prod_us_c1_deployment = get_deployment();

        edge_eu_w4_deployment.name = "edge_eu_w4_deployment".to_string();
        stage_eu_w4_deployment.name = "stage_eu_w4_deployment".to_string();
        prod_as_e1_deployment.name = "prod_as_e1_deployment".to_string();
        prod_eu_w4_deployment.name = "prod_eu_w4_deployment".to_string();
        prod_us_c1_deployment.name = "prod_us_c1_deployment".to_string();

        cfg.deployments = vec![
            edge_eu_w4_deployment,
            stage_eu_w4_deployment,
            prod_as_e1_deployment,
            prod_eu_w4_deployment,
            prod_us_c1_deployment,
        ];

        let mut cmd = get_cmd();
        cmd.opts.filter = Option::from("^prod".to_string());

        let res = cmd.plan(cfg).unwrap();
        let prod_as_e1_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";
        let prod_eu_w4_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";
        let prod_us_c1_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";

        let prod_as_e1_deployment_expected_helm_cmd: Vec<String> =
            prod_as_e1_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let prod_eu_w4_deployment_expected_helm_cmd: Vec<String> =
            prod_eu_w4_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let prod_us_c1_deployment_expected_helm_cmd: Vec<String> =
            prod_us_c1_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let got_as_e1 = res.commands.get("prod_as_e1_deployment").unwrap();
        let got_eu_w4 = res.commands.get("prod_eu_w4_deployment").unwrap();
        let got_us_c1 = res.commands.get("prod_us_c1_deployment").unwrap();

        assert_eq!(prod_as_e1_deployment_expected_helm_cmd, got_as_e1.1);
        assert_eq!(prod_eu_w4_deployment_expected_helm_cmd, got_eu_w4.1);
        assert_eq!(prod_us_c1_deployment_expected_helm_cmd, got_us_c1.1);
        assert_eq!(res.commands.len(), 3);
    }

    #[test]
    fn filter_all_eu_w4_deployment() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut edge_eu_w4_deployment = get_deployment();
        let mut stage_eu_w4_deployment = get_deployment();
        let mut prod_as_e1_deployment = get_deployment();
        let mut prod_eu_w4_deployment = get_deployment();
        let mut prod_us_c1_deployment = get_deployment();

        edge_eu_w4_deployment.name = "edge_eu_w4_deployment".to_string();
        stage_eu_w4_deployment.name = "stage_eu_w4_deployment".to_string();
        prod_as_e1_deployment.name = "prod_as_e1_deployment".to_string();
        prod_eu_w4_deployment.name = "prod_eu_w4_deployment".to_string();
        prod_us_c1_deployment.name = "prod_us_c1_deployment".to_string();

        cfg.deployments = vec![
            edge_eu_w4_deployment,
            stage_eu_w4_deployment,
            prod_as_e1_deployment,
            prod_eu_w4_deployment,
            prod_us_c1_deployment,
        ];

        let mut cmd = get_cmd();
        cmd.opts.filter = Option::from("eu_w4".to_string());

        let res = cmd.plan(cfg).unwrap();

        let edge_eu_w4_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";
        let prod_eu_w4_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";
        let stage_eu_w4_deployment_expected_helm_cmd =
            "helm template some-release charts/some-chart --namespace=default";

        let edge_eu_w4_deployment_expected_helm_cmd: Vec<String> =
            edge_eu_w4_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let prod_eu_w4_deployment_expected_helm_cmd: Vec<String> =
            prod_eu_w4_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let stage_eu_w4_deployment_expected_helm_cmd: Vec<String> =
            stage_eu_w4_deployment_expected_helm_cmd
                .split_whitespace()
                .map(String::from)
                .collect();

        let got_edge = res.commands.get("edge_eu_w4_deployment").unwrap();
        let got_stage = res.commands.get("stage_eu_w4_deployment").unwrap();
        let got_prod = res.commands.get("prod_eu_w4_deployment").unwrap();

        assert_eq!(edge_eu_w4_deployment_expected_helm_cmd, got_edge.1);
        assert_eq!(prod_eu_w4_deployment_expected_helm_cmd, got_prod.1);
        assert_eq!(stage_eu_w4_deployment_expected_helm_cmd, got_stage.1);
        assert_eq!(res.commands.len(), 3);
    }

    #[test]
    fn pipe_output_through_tool() {
        let mut cfg = get_config();
        cfg.chart = PathBuf::from("charts/some-chart");
        cfg.namespace = Option::from("default".to_string());
        cfg.release_name = "some-release".to_string();
        cfg.output_path = PathBuf::from("manifests");

        let mut edge = get_deployment();
        edge.name = "edge".to_string();

        cfg.deployments = vec![edge];

        let mut cmd = get_cmd();
        let pipe_command = vec!["grep images".to_string()];

        cmd.opts.pipe = Option::from(pipe_command);

        let res = cmd.plan(cfg).unwrap();

        let base_helm_cmd = "helm template some-release charts/some-chart --namespace=default";

        let mut edge_expected_helm_cmd: Vec<String> =
            base_helm_cmd.split_whitespace().map(String::from).collect();

        edge_expected_helm_cmd.push("| grep images".to_owned());

        let got_edge = res.commands.get("edge").unwrap();

        assert_eq!(edge_expected_helm_cmd, got_edge.1);
        assert_eq!(res.commands.len(), 1);
    }
}
