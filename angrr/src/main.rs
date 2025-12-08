use angrr::{
    command::{Commands, Options},
    config::Config,
    run::RunContext,
    touch::TouchContext,
};
use anyhow::Context;
use clap::{Parser, crate_name};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../etc/"]
struct Etc;

fn main() -> anyhow::Result<()> {
    let crate_name = crate_name!();

    let options = Options::parse();
    let default_config_file = Etc::get("angrr.toml")
        .context("failed to get default config file from embedded resources")?;
    let default_config_str = std::str::from_utf8(&default_config_file.data)
        .context("failed to convert default config file to UTF-8 string")?;
    let mut figment = Figment::new().merge(Toml::string(default_config_str));
    if let Some(path) = options.common.config.as_ref() {
        figment = figment.merge(Toml::file(path));
    }
    let config: Config = figment.merge(Env::prefixed("ANGRR_")).extract()?;

    config.validate()?;

    let mut logger_builder = pretty_env_logger::formatted_builder();

    // default logger configuration
    let default_log_level = log::LevelFilter::Info;
    logger_builder.filter_module(crate_name, default_log_level);
    // overrides
    if let Ok(filter) = std::env::var("RUST_LOG") {
        logger_builder.parse_filters(&filter);
    };
    // configuration file
    logger_builder.filter_module(crate_name, config.log_level);
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
    let builder_debug_info = format!("{logger_builder:?}");
    logger_builder.try_init()?;
    log::debug!("logger initialized with configuration: {builder_debug_info}");

    log::debug!("parsed options: {options:#?}");
    log::debug!("loaded config: {config:#?}");

    match options.command {
        Commands::Run(run_opts) => {
            let context = RunContext::new(run_opts, config)?;
            log::trace!("context = {context:#?}");
            context.run()?;
            context.finish()?;
        }
        Commands::Touch(touch_opts) => {
            let context = TouchContext::new(touch_opts, config);
            log::trace!("context = {context:#?}");
            context.touch()?;
        }
    }
    Ok(())
}
