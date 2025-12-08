use clap::{Parser, Subcommand, ValueEnum};

use core::fmt;
use std::{ffi::OsString, path::PathBuf};

const HELP_TEMPLATE: &str = "\
{before-help}{name} {version}
{author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
";

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(
    propagate_version = true,
    infer_long_args = true,
    infer_subcommands = true,
    flatten_help = true
)]
#[command(help_template = HELP_TEMPLATE)]
pub struct Options {
    #[command(subcommand)]
    pub command: Commands,
    #[command(flatten)]
    pub common: CommonOptions,
}

#[derive(Clone, Debug, Parser)]
pub struct CommonOptions {
    /// Path to settings file
    #[arg(global = true, short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Increase log level (will be overridden by --log-level).
    #[arg(global = true, short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Run(RunOptions),
    Touch(TouchOptions),
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Do retention")]
pub struct RunOptions {
    /// Prompt according to WHEN: never, once, or always
    ///
    /// `-i` or `--interactive` means `--interactive=always`.
    #[arg(
        short,
        long,
        value_name = "WHEN",
        default_value_t = Interactive::Once,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "always",
        default_value_if("no_prompt", "true", "never")
    )]
    pub interactive: Interactive,

    /// Never prompt flag, override by `--interactive`.
    #[arg(short, long)]
    pub no_prompt: bool,

    /// Do not output statistic data.
    #[arg(long)]
    pub no_statistic: bool,

    /// Output removed paths to the file.
    ///
    /// When FILE is -, write to standard output.
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Disable extra output buffering.
    #[arg(long)]
    pub output_unbuffered: bool,

    /// Output delimiter
    #[arg(
        long,
        value_name = "DELIMITER",
        default_value_os_t = OsString::from("\n"),
        default_value_if("null_output_delimiter", "true", "\0")
    )]
    pub output_delimiter: OsString,

    /// Use `\0` character as the output delimiter.
    #[arg(long)]
    pub null_output_delimiter: bool,

    /// Do not remove file.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Touch GC roots")]
#[command(arg_required_else_help = true)]
pub struct TouchOptions {
    /// Path to touch
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// "Do not recurse into directories.
    #[arg(short, long)]
    pub no_recursive: bool,

    /// Do not output to stdout.
    #[arg(short, long)]
    pub silent: bool,

    /// Do not actually touch files.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Interactive {
    Never,
    Once,
    Always,
}

impl fmt::Display for Interactive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Interactive::Never => write!(f, "never"),
            Interactive::Once => write!(f, "once"),
            Interactive::Always => write!(f, "always"),
        }
    }
}
