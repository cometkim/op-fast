use std::ffi::OsString;

use anyhow::Result;
use clap::Parser;

pub mod cache;
pub mod inject;
pub mod read;
pub mod run;

#[derive(Debug, Parser)]
pub enum Subcommand {
    Read(read::ReadArgs),
    Inject(inject::InjectArgs),
    Run(run::RunArgs),
    Cache(cache::CacheCommand),

    #[clap(external_subcommand)]
    Other(Vec<OsString>),
}

pub fn execute(subcommand: Subcommand) -> Result<()> {
    match subcommand {
        Subcommand::Read(args) => read::execute(args),
        Subcommand::Inject(args) => inject::execute(args),
        Subcommand::Run(args) => run::execute(args),
        Subcommand::Cache(cmd) => cache::execute(cmd),
        Subcommand::Other(args) => delegate_passthrough(&args),
    }
}

fn delegate_passthrough(args: &[OsString]) -> Result<()> {
    let delegate = crate::delegate::OpDelegate::new()?;
    let status = delegate.command().args(args).status()?;

    std::process::exit(status.code().unwrap_or(1));
}
