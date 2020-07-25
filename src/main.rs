mod config;
mod render_cmd;
mod validate_cmd;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

use crate::render_cmd::RenderCmd;
use anyhow::{Context, Result};
use env_logger::Builder;
use log::LevelFilter;
use std::path::PathBuf;
use structopt::StructOpt;
use structopt_flags::GetWithDefault;
use validate_cmd::ValidateCmd;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "helm-templexer",
    about = "Render Helm charts for multiple environments using explicit config"
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
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let log_level = args.verbose.get_with_default(LevelFilter::Info);
    Builder::from_default_env().filter_level(log_level).init();

    match args.cmd {
        SubCmd::Validate(opts) => ValidateCmd::new(opts)
            .run()
            .context("Configuration failed validation")?,
        SubCmd::Render(opts) => RenderCmd::new(opts).run().context("Rendering failed")?,
    };

    Ok(())
}
