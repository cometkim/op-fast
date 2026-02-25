use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueHint};

use crate::delegate::OpDelegate;
use crate::store::Store;

fn parse_file_mode(s: &str) -> Result<u32> {
    u32::from_str_radix(s, 8).context("Invalid file mode (expected octal)")
}

#[derive(Debug, Parser)]
#[clap(
    about = "Read a secret reference",
    long_about = "Read the value of the field in 1Password specified by a secret reference.

Secrets are cached in the OS keyring for offline access with configurable TTL.

Secret reference syntax: op://<vault>/<item>/<field>

Learn more about secret references:
https://developer.1password.com/docs/cli/secrets-reference-syntax/",
    after_help = "Examples:
  Read a password:
    op-fast read op://app-prod/db/password

  Read with variable substitution:
    VAULT=prod op-fast read 'op://$VAULT/db/password'

  Save to a file with restricted permissions:
    op-fast read -o ./key.pem op://app-prod/ssh/private-key
"
)]
pub struct ReadArgs {
    #[clap(help = "Secret reference (e.g., op://vault/item/field)")]
    pub reference: String,

    #[clap(
        long = "file-mode",
        value_name = "filemode",
        value_parser = parse_file_mode,
        default_value = "600",
        help = "Set file mode for the output file (octal, ignored without --out-file)"
    )]
    pub file_mode: u32,

    #[clap(short = 'f', long = "force", help = "Do not prompt for confirmation")]
    pub force: bool,

    #[clap(
        short = 'n',
        long = "no-newline",
        help = "Do not print a newline after the secret"
    )]
    pub no_newline: bool,

    #[clap(
        short = 'o',
        long = "out-file",
        value_name = "FILE",
        value_hint = ValueHint::FilePath,
        help = "Write the secret to a file instead of stdout"
    )]
    pub out_file: Option<PathBuf>,
}

pub fn execute(args: ReadArgs) -> Result<()> {
    let store = Store::open();
    let delegate = OpDelegate::new()?;

    let value = match store {
        Ok(store) => match store.get(&args.reference)? {
            Some(value) => value,
            None => {
                let value = delegate.read(&args.reference)?;
                store.put(&args.reference, &value)?;
                value
            }
        },
        Err(e) => {
            log::error!("Store unavailable, delegating to op: {}", e);
            delegate.read(&args.reference)?
        }
    };

    if let Some(out_file) = &args.out_file {
        let mut file = File::create(out_file)
            .with_context(|| format!("Failed to create file: {:?}", out_file))?;
        file.write_all(value.as_bytes())?;

        use std::fs;
        fs::set_permissions(out_file, PermissionsExt::from_mode(args.file_mode))
            .with_context(|| format!("Failed to set file mode: {:o}", args.file_mode))?;
    } else if args.no_newline {
        print!("{}", value);
    } else {
        println!("{}", value);
    }

    Ok(())
}
