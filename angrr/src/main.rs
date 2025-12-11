use std::io::Write;

use angrr::config::{RunConfig, load_config};
use angrr::embedded;
use angrr::{
    command::{Commands, Options},
    run::RunContext,
    touch::TouchContext,
};
use anyhow::Context;
use clap::{Parser, crate_name};

fn main() -> anyhow::Result<()> {
    let crate_name = crate_name!();

    let options = Options::parse();

    let mut logger_builder = pretty_env_logger::formatted_builder();

    // default logger configuration
    let default_log_level = log::LevelFilter::Info;
    logger_builder.filter_module(crate_name, default_log_level);
    // overrides
    if let Ok(filter) = std::env::var("RUST_LOG") {
        logger_builder.parse_filters(&filter);
    };
    // option
    if options.common.verbose != 0 {
        let mut iter = log::LevelFilter::iter().fuse();
        // find the default log level
        iter.find(|level| *level == default_log_level);
        for _ in 0..(options.common.verbose - 1) {
            iter.next();
        }
        // since our iter is a Fuse, it must return None if we already reach the max
        // level
        let level = match iter.next() {
            Some(l) => l,
            None => log::LevelFilter::max(),
        };
        logger_builder.filter_module(crate_name, level);
    }
    // configuration file
    if let Some(filter) = options.common.log_level {
        logger_builder.filter_module(crate_name, filter);
    }
    let builder_debug_info = format!("{logger_builder:?}");
    logger_builder.try_init()?;
    log::debug!("logger initialized with configuration: {builder_debug_info}");

    log::debug!("parsed options: {options:#?}");

    match options.command {
        Commands::Run(run_opts) => {
            let config = load_config(&options.common.config)?;
            log::info!(
                "loaded config:\n{}",
                angrr::config::display_config(&config)?
            );
            let context = RunContext::new(run_opts, config)?;
            log::trace!("context = {context:#?}");
            context.run()?;
            context.finish()?;
        }
        Commands::Validate => {
            let config: RunConfig = load_config(&options.common.config)?;
            println!("{}", angrr::config::display_config(&config)?)
        }
        Commands::Touch(touch_opts) => {
            let config = load_config(&options.common.config)?;
            let context = TouchContext::new(touch_opts, config);
            log::trace!("context = {context:#?}");
            context.touch()?;
        }
        Commands::ExampleConfig => {
            let example = embedded::Etc::get("example-config.toml")
                .context("failed to extract embedded example configuration file")?;
            std::io::stdout()
                .write_all(&example.data)
                .context("failed to write example configuration to stdout")?;
            std::io::stdout()
                .flush()
                .context("failed to flush stdout")?;
        }
    }
    Ok(())
}
