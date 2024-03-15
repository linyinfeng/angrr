use clap::{Parser, Subcommand, ValueEnum};

use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
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
pub struct RunOptions {
    #[arg(
        short,
        long,
        default_value = "/nix/var/nix/gcroots/auto",
        help = "directories containing auto GC roots"
    )]
    pub directory: Vec<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration, help = "retention period")]
    pub period: Duration,
    #[arg(
        long,
        value_name = "WHEN",
        help = "prompt according to WHEN: never, once, or always",
        default_value = "once",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "always",
        default_value_if("no_prompt", "true", "never")
    )]
    pub interactive: Interactive,
    #[arg(short, long, help = "never prompt, override by --interactive")]
    pub no_prompt: bool,
    #[arg(short = 't', long, help = "remove target instead of the GC root")]
    pub remove_target: bool,
    #[arg(
        short,
        long,
        help = "only delete owned GC roots\nshould be used with --remove-garget for non-root users"
    )]
    pub owned_only: bool,
    #[arg(long, help = "include GC roots whose target has been removed")]
    pub include_not_found: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Generate shell completions")]
pub struct CompletionOptions {
    pub shell: clap_complete::Shell,
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Interactive {
    Never,
    Once,
    Always,
}
