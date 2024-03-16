use clap::{ArgAction, Parser, Subcommand, ValueEnum};

use std::{ffi::OsString, path::PathBuf, time::Duration};

const HELP_TEMPLATE: &'static str = "\
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
}

#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Run(RunOptions),
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
        default_value = "/nix/var/nix/gcroots/auto",
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
        default_value = "once",
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
        default_value = "true",
        default_value_if("remove_root", "true", "false")
    )]
    pub owned_only: bool,
    #[arg(long, help = "do not output statistic data")]
    pub no_statistic: bool,
    #[arg(long, value_name = "FILE", help = "output removed paths to file")]
    pub output: Option<PathBuf>,
    #[arg(
        long,
        value_name = "DELIMITER",
        help = "output delimiter",
        default_value = "\n",
        default_value_if("null_output_delimiter", "true", "\0")
    )]
    pub output_delimiter: OsString,
    #[arg(long, help = "use \"\\0\" as the output delimiter")]
    pub null_output_delimiter: bool,
    #[arg(long)]
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
