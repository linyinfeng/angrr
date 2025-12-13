use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use clap_complete::aot::{Bash, Fish, Zsh};
use xshell::cmd;

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
    let sh = xshell::Shell::new()?;
    let commands = Commands::parse();
    match commands {
        Commands::ManPages(gen_options) => generate_man_pages(gen_options, &sh),
        Commands::ShellCompletions(gen_options) => generate_shell_completions(gen_options),
    }
}

fn generate_man_pages(gen_options: ManPageOptions, sh: &xshell::Shell) -> anyhow::Result<()> {
    let out = &gen_options.out;

    let config_md = sh.read_file("docs/config.md")?;
    let example_config = sh.read_file("etc/example-config.toml")?;
    let final_config_md = config_md.replace("EXAMPLE_CONFIG_PLACEHOLDER", &example_config);
    let temp_dir = sh.create_temp_dir()?;
    let final_config_md_path = temp_dir.path().join("config.md");
    sh.write_file(&final_config_md_path, final_config_md)?;
    cmd!(
        sh,
        "go-md2man -in {final_config_md_path} -out {out}/angrr.5"
    )
    .run()?;

    clap_mangen::generate_to(angrr::command::Options::command(), out)?;
    Ok(())
}

fn generate_shell_completions(gen_options: CompletionOptions) -> anyhow::Result<()> {
    let mut cli = angrr::command::Options::command();
    let out = &gen_options.out;
    clap_complete::generate_to(Bash, &mut cli, "angrr", out)?;
    clap_complete::generate_to(Fish, &mut cli, "angrr", out)?;
    clap_complete::generate_to(Zsh, &mut cli, "angrr", out)?;
    Ok(())
}
