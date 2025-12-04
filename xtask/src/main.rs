use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::aot::{Bash, Fish, Zsh};

#[derive(Clone, Debug, Parser)]
pub enum Commands {
    ManPages(ManPageOptions),
    ShellCompletions(CompletionOptions),
}

#[derive(Clone, Debug, Parser)]
pub struct ManPageOptions {
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Clone, Debug, Parser)]
pub struct CompletionOptions {
    #[arg(long)]
    pub out: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let commands = Commands::parse();
    match commands {
        Commands::ManPages(gen_options) => generate_man_pages(gen_options),
        Commands::ShellCompletions(gen_options) => generate_shell_completions(gen_options),
    }
}

fn generate_man_pages(gen_options: ManPageOptions) -> anyhow::Result<()> {
    clap_mangen::generate_to(angrr::options::Options::command(), &gen_options.out)?;
    Ok(())
}

fn generate_shell_completions(gen_options: CompletionOptions) -> anyhow::Result<()> {
    let mut cli = angrr::options::Options::command();
    let out = &gen_options.out;
    clap_complete::generate_to(Bash, &mut cli, "angrr", out)?;
    clap_complete::generate_to(Fish, &mut cli, "angrr", out)?;
    clap_complete::generate_to(Zsh, &mut cli, "angrr", out)?;
    Ok(())
}
