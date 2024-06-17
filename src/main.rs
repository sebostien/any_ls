use std::process::ExitCode;

use clap::Parser;
use log::LevelFilter;

fn main() -> ExitCode {
    let args = any_ls::Cli::parse();
    init_log(args.verbosity);

    if let Err(e) = any_ls::start(args) {
        eprintln!("{e}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn init_log(verbosity: u8) {
    let level_filter = match verbosity {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::max(),
    };

    env_logger::Builder::from_env("ANY_LS_LOG")
        .filter(None, level_filter)
        .default_format()
        .format_timestamp_secs()
        .init();
}
