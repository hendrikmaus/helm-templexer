mod config;
mod validate_cmd;

#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;

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
    #[structopt(name = "validate", about = "Validate given configuration file")]
    Validate(ValidateCmdOpts),
}

#[derive(StructOpt, Debug)]
pub struct ValidateCmdOpts {
    // TODO support passing in multiple files?
    /// Configuration file to validate (supported formats: toml, yaml, json)
    input_file: PathBuf,

    #[structopt(short, long, about = "Skip validation if `enabled` is set to false")]
    skip_disabled: bool,
}

fn main() -> Result<()> {
    let args = Args::from_args();

    let log_level = args.verbose.get_with_default(LevelFilter::Info);
    Builder::from_default_env().filter_level(log_level).init();

    match args.cmd {
        SubCmd::Validate(opts) => ValidateCmd::new(opts)
            .run()
            .context("Configuration failed validation")?,
    };

    Ok(())
}
