use anyhow::Context;
use std::path::PathBuf;
use structopt::{
    clap::AppSettings::{ColoredHelp, GlobalVersion, VersionlessSubcommands},
    StructOpt,
};
use structopt_flags::GetWithDefault;

use validate_cmd::ValidateCmd;

use crate::render_cmd::RenderCmd;

mod config;
mod render_cmd;
mod validate_cmd;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "helm-templexer",
    about = "Render Helm charts for multiple environments using explicit config",
    global_settings = &[ColoredHelp, VersionlessSubcommands, GlobalVersion]
)]
struct Args {
    #[structopt(flatten)]
    verbose: structopt_flags::VerboseNoDef,

    #[structopt(subcommand)]
    cmd: SubCmd,
}

#[derive(StructOpt, Debug)]
enum SubCmd {
    #[structopt(name = "validate", about = "Validate given configuration file(s)")]
    Validate(ValidateCmdOpts),

    #[structopt(
        name = "render",
        about = "Render deployments for given configuration file(s)"
    )]
    Render(RenderCmdOpts),
}

#[derive(StructOpt, Debug)]
pub struct ValidateCmdOpts {
    /// Configuration file(s) to validate (supported formats: toml, yaml, json)
    input_files: Vec<PathBuf>,

    #[structopt(short, long, about = "Skip validation if `enabled` is set to false")]
    skip_disabled: bool,
}

#[derive(StructOpt, Debug)]
pub struct RenderCmdOpts {
    /// Configuration file(s) to render deployments for (supported formats: toml, yaml, json)
    input_files: Vec<PathBuf>,

    /// (not implemented) Optional helm binary to use; defaults to the binary found in the PATH or fails, if none is found
    #[structopt(short, long)]
    helm_bin: Option<PathBuf>,

    /// Pass additional options to the underlying 'helm template' call, e.g. '--set-string image.tag=${revision}'
    #[structopt(short, long, multiple = true)]
    additional_options: Option<Vec<String>>,

    /// Print rendered manifests to stdout
    #[structopt(short, long)]
    stdout: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let log_level = args.verbose.get_with_default(log::LevelFilter::Info);
    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .init();

    match args.cmd {
        SubCmd::Validate(opts) => ValidateCmd::new(opts)
            .run()
            .context("Configuration failed validation")?,
        SubCmd::Render(opts) => RenderCmd::new(opts).run().context("Rendering failed")?,
    };

    Ok(())
}
