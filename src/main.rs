#[macro_use]
extern crate log;

use env_logger::Builder;
use log::LevelFilter;
use structopt::StructOpt;
use structopt_flags::GetWithDefault;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "helm-templexer",
    about = "Render Helm charts for multiple environments using explicit config"
)]
struct Args {
    #[structopt(flatten)]
    verbose: structopt_flags::VerboseNoDef,
}

fn main() {
    let args = Args::from_args();

    let log_level = args.verbose.get_with_default(LevelFilter::Info);
    Builder::from_default_env().filter_level(log_level).init();

    info!("hello, world!");
}
