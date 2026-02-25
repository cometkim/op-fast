use clap::Parser;

mod cache;
mod command;
mod config;
mod delegate;
mod template;

#[derive(Debug, Parser)]
#[clap(
    name = "op-offline",
    about = "Offline cache for 1Password CLI",
    version
)]
struct Cli {
    #[clap(subcommand)]
    subcommand: command::Subcommand,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    cache::init()?;

    let cli = Cli::parse();
    command::execute(cli.subcommand)
}
