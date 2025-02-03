use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(version)]
pub struct CliArgs {
    #[arg()]
    pub folder: PathBuf,

    #[arg(short, long, default_value = "0.0.0.0")]
    pub addr: String,

    #[arg(short, long, default_value = "0")]
    pub port: u16,

    /// The verbosity of the logger
    #[cfg(not(debug_assertions))]
    #[arg(value_enum, short, long, default_value_t = LogLevel::Warn)]
    pub verbosity: LogLevel,

    /// The verbosity of the logger
    #[cfg(debug_assertions)]
    #[arg(value_enum, short, long, default_value_t = LogLevel::Info)]
    pub verbosity: LogLevel,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    #[cfg(debug_assertions)]
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,

            #[cfg(debug_assertions)]
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}
