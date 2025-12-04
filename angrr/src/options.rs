use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use regex::bytes::Regex;

use core::fmt;
use std::{ffi::OsString, path::PathBuf, time::Duration};

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
    /// Increase log level (will be overridden by --log-level).
    #[arg(global = true, short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log level (off, error, warn, info, debug, trace)
    #[arg(global = true, long, value_name = "LEVEL")]
    pub log_level: Option<log::LevelFilter>,

    /// Store path for validation
    #[arg(
        long,
        value_name = "PATH",
        default_value_os_t = PathBuf::from("/nix/store")
    )]
    pub store: PathBuf,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Run(RunOptions),
    Touch(TouchOptions),
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Do retention")]
#[command(arg_required_else_help = true)]
pub struct RunOptions {
    /// Directories containing auto GC roots
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_values_os_t = [PathBuf::from("/nix/var/nix/gcroots/auto")],
    )]
    pub directory: Vec<PathBuf>,

    /// Retention period
    #[arg(short, long,
        value_name = "DURATION", value_parser = humantime::parse_duration)]
    pub period: Duration,

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

    /// Remove GC root instead of the symbolic link target of the root.
    ///
    /// Also set `--owned-only=false` as default value of `--owned-only`.
    #[arg(short, long)]
    pub remove_root: bool,

    /// Only delete owned symbolic link target of GC roots.
    #[arg(
        long,
        value_name = "BOOL",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_value_t = true,
        default_value_if("remove_root", "true", "false")
    )]
    pub owned_only: bool,

    /// Path prefixes to ignore
    #[arg(
        long,
        visible_alias = "ignore-directories",
        value_name = "PATH",
        default_values_os_t = [PathBuf::from("/nix/var/nix/profiles")]
    )]
    pub ignore_prefixes: Vec<PathBuf>,

    /// Path prefixes (relative to user's home) to ignore
    #[arg(
        long,
        visible_alias = "ignore-directories-in-home",
        value_name = "PATH",
        default_values_os_t = [
            PathBuf::from(".local/state/nix/profiles"),
            PathBuf::from(".local/state/home-manager/gcroots"),
            PathBuf::from(".cache/nix/flake-registry.json")
        ],
    )]
    pub ignore_prefixes_in_home: Vec<PathBuf>,

    /// Only paths (absolute) matching the regex will be monitored
    #[arg(
        long,
        value_name = "REGEX",
        default_value_t = Regex::new(r"/\.direnv/|/result.*$").unwrap()
    )]
    pub path_regex: Regex,

    /// An external program to filter paths that will be applied after all the other filter options
    ///
    /// A JSON object containing the path information will be passed to the stdin of the program,
    /// use `--filter=cat --verbose` to view the input json object.
    /// If the program exits with code 0, then the path will be monitored; otherwise it will be ignored.
    #[arg(long, value_name = "EXECUTABLE")]
    pub filter: Option<PathBuf>,

    /// Arguments to pass to the external filter program
    #[arg(long, value_name = "ARGUMENTS")]
    pub filter_args: Vec<OsString>,

    /// Force delete targets of GC roots that do not point to store.
    ///
    /// Validation only happens when `--remove-root` is not specified.
    #[arg(long)]
    pub force: bool,

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
