use clap::{Parser, Subcommand};

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
    #[arg(short, long, default_value = "/nix/var/nix/gcroots/auto")]
    pub directory: Vec<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration)]
    pub period: Duration,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Parser)]
#[command(about = "Generate shell completions")]
pub struct CompletionOptions {
    pub shell: clap_complete::Shell,
}
