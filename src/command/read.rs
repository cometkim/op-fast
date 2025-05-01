use std::path::PathBuf;

use clap::{Parser, ValueHint};

#[derive(Debug, Parser)]
pub(crate) struct ReadArgs {
    /// Secret reference, e.g. op://app-prod/db/password
    pub reference: String,

    /// Set filemode for the output file. Ignored without --out-file flag
    #[clap(long = "file-mode", value_name = "filemode")]
    pub file_mode: Option<u32>,

    /// Do not prompt for confirmation
    #[clap(short = 'f', long = "force")]
    pub force: bool,

    /// Do not print a new line after the secret
    #[clap(short = 'n', long = "no-newline")]
    pub no_newline: bool,

    /// Write the secret to a file instead of stdout
    #[clap(
        short = 'o',
        long = "out-file",
        value_name = "FILE",
        value_hint = ValueHint::FilePath
    )]
    pub out_file: Option<PathBuf>,
}
