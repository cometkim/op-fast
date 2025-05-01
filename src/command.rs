use std::ffi::OsString;

use clap::Subcommand;

use self::inject::InjectArgs;
use self::read::ReadArgs;
use self::run::RunArgs;

mod inject;
mod read;
mod run;

#[derive(Subcommand)]
pub(crate) enum Command {
    Inject(InjectArgs),
    Read(ReadArgs),
    Run(RunArgs),

    #[clap(external_subcommand)]
    Other(Vec<OsString>),
}
