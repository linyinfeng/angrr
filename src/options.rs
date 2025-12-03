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
    #[arg(global = true, short, long, action = clap::ArgAction::Count, help = "increase log level (will be overridden by --log-level)")]
    pub verbose: u8,
    #[arg(
        global = true,
        long,
        help = "set log level (off, error, warn, info, debug, trace)",
        value_name = "LEVEL"
    )]
    pub log_level: Option<log::LevelFilter>,
    #[arg(
        long,
        value_name = "PATH",
        help = "store path for validation",
        default_value_os_t = PathBuf::from("/nix/store")
    )]
    pub store: PathBuf,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Run(RunOptions),
    Touch(TouchOptions),
    Completion(CompletionOptions),
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Do retention")]
#[command(arg_required_else_help = true)]
pub struct RunOptions {
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_values_os_t = [PathBuf::from("/nix/var/nix/gcroots/auto")],
        help = "directories containing auto GC roots"
    )]
    pub directory: Vec<PathBuf>,
    #[arg(short, long,
        value_name = "DURATION", value_parser = humantime::parse_duration, help = "retention period")]
    pub period: Duration,
    #[arg(
        short,
        long,
        value_name = "WHEN",
        help = "\
prompt according to WHEN: never, once, or always
`-i` or `--interactive` means `--interactive=always`
", // add a new line for default and possible values in help
        default_value_t = Interactive::Once,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "always",
        default_value_if("no_prompt", "true", "never")
    )]
    pub interactive: Interactive,
    #[arg(short, long, help = "never prompt, override by --interactive")]
    pub no_prompt: bool,
    #[arg(
        short,
        long,
        help = "\
remove GC root instead of the symbolic link target of the root
also set `--owned-only=false` as default value of `--owned-only`"
    )]
    pub remove_root: bool,
    #[arg(
        long,
        value_name = "BOOL",
        // add a new line for default and possible values in help
        help = "only delete owned symbolic link target of GC roots\n",
        action = ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_value_t = true,
        default_value_if("remove_root", "true", "false")
    )]
    pub owned_only: bool,
    #[arg(
        long,
        visible_alias = "ignore-directories",
        value_name = "PATH",
        default_values_os_t = [PathBuf::from("/nix/var/nix/profiles")],
        help = "directories to ignore"
    )]
    pub ignore_prefixes: Vec<PathBuf>,
    #[arg(
        long,
        visible_alias = "ignore-directories-in-home",
        value_name = "PATH",
        default_values_os_t = [
            PathBuf::from(".local/state/nix/profiles"),
            PathBuf::from(".local/state/home-manager/gcroots"),
            PathBuf::from(".cache/nix/flake-registry.json")
        ],
        help = "directories (relative to user's home) to ignore"
    )]
    pub ignore_prefixes_in_home: Vec<PathBuf>,
    #[arg(
        long,
        value_name = "REGEX",
        default_value_t = Regex::new(r"/\.direnv/|/result.*$").unwrap(),
        help = "only paths (absolute) matching the regex will be monitored",
    )]
    pub path_regex: Regex,
    #[arg(
        long,
        value_name = "EXECUTABLE",
        help = "\
an external program to filter paths that will be applied after all the other filter options
a json object containing the path information will be passed to the stdin of the program,
use `--filter=cat --verbose` to view the input json object.
if the program exits with code 0, then the path will be monitored; otherwise it will be ignored"
    )]
    pub filter: Option<PathBuf>,
    #[arg(
        long,
        value_name = "ARGUMENTS",
        help = "arguments to pass to the external filter program"
    )]
    pub filter_args: Vec<OsString>,
    #[arg(
        long,
        help = "\
force delete targets of GC roots that do not point to store
validation only happens when `--remove-root` is not specified"
    )]
    pub force: bool,
    #[arg(long, help = "do not output statistic data")]
    pub no_statistic: bool,
    #[arg(
        long,
        value_name = "FILE",
        help = "\
output removed paths to file,
when FILE is -, write to standard output"
    )]
    pub output: Option<PathBuf>,
    #[arg(long, help = "disable extra output buffer")]
    pub output_unbuffered: bool,
    #[arg(
        long,
        value_name = "DELIMITER",
        help = "output delimiter",
        default_value_os_t = OsString::from("\n"),
        default_value_if("null_output_delimiter", "true", "\0")
    )]
    pub output_delimiter: OsString,
    #[arg(long, help = "use \"\\0\" as the output delimiter")]
    pub null_output_delimiter: bool,
    #[arg(long, help = "do not remove file")]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Touch GC roots")]
#[command(arg_required_else_help = true)]
pub struct TouchOptions {
    #[arg(value_name = "PATH")]
    pub path: PathBuf,
    #[arg(short, long)]
    pub no_recursive: bool,
    #[arg(short, long)]
    pub silent: bool,
    #[arg(long, help = "do not actually touch file")]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Generate shell completions")]
#[command(arg_required_else_help = true)]
pub struct CompletionOptions {
    pub shell: clap_complete::Shell,
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
