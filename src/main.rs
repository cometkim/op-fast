use clap::Parser;

mod command;
mod config;
mod delegate;
mod store;
mod template;

#[derive(Debug, Parser)]
#[clap(
    name = "op-fast",
    version,
    disable_help_subcommand = true,
    subcommand_required = true,
    arg_required_else_help = true,
    override_usage = "op-fast <command> [flags]",
    next_display_order = None,
    about = "1Password CLI proxy for instant access to secrets",
    long_about = "1Password CLI proxy for instant access to secrets

It caches 1Password secret references in the OS keyring with configurable TTL.
Provides instant access to previously fetched secrets without requiring
re-authentication or network roundtrips.

Commands:
  read    Read a secret reference
  inject  Inject secrets into a config file
  run     Pass secrets as environment variables to a process

Custom Commands:
  store   Manage op-fast store

All other commands are passed through to the real 'op' binary."
)]
struct Cli {
    #[clap(subcommand)]
    subcommand: command::Subcommand,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    store::init()?;

    let cli = Cli::parse();
    command::execute(cli.subcommand)
}
