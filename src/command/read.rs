use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueHint};

use crate::delegate::OpDelegate;
use crate::store::Store;

#[derive(Debug, Parser)]
pub struct ReadArgs {
    pub reference: String,

    #[clap(long = "file-mode", value_name = "filemode")]
    pub file_mode: Option<u32>,

    #[clap(short = 'f', long = "force")]
    pub force: bool,

    #[clap(short = 'n', long = "no-newline")]
    pub no_newline: bool,

    #[clap(
        short = 'o',
        long = "out-file",
        value_name = "FILE",
        value_hint = ValueHint::FilePath
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
    } else if args.no_newline {
        print!("{}", value);
    } else {
        println!("{}", value);
    }

    Ok(())
}
