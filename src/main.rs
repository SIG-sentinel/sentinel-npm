use clap::Parser;
use sentinel::artifact_store_config;
use sentinel::commands;
use sentinel::constants::{
    DEFAULT_LOG_FILTER, ENV_RUST_LOG, ENV_SENTINEL_LOG, MAIN_ERR_ARTIFACT_STORE_INIT_FAILED,
    MAIN_ERR_SIGPIPE_INIT_FAILED,
};
use sentinel::types::{Cli, Commands};
use sentinel::verifier::artifact_cleanup;
use std::process::ExitCode;

#[cfg(unix)]
use std::sync::{Arc, atomic::AtomicBool};

const DEFAULT_FALLBACK_LOG_FILTER: &str = "info";

#[cfg(unix)]
fn init_sigpipe() -> Result<(), std::io::Error> {
    let should_use_default_handler = Arc::new(AtomicBool::new(true));

    signal_hook::flag::register_conditional_default(
        signal_hook::consts::signal::SIGPIPE,
        should_use_default_handler,
    )
    .map(|_| ())
}

fn init_logging() {
    let sentinel_log_enabled = std::env::var(ENV_SENTINEL_LOG).is_ok();
    let rust_log_enabled = std::env::var(ENV_RUST_LOG).is_ok();
    let has_logging_env = sentinel_log_enabled || rust_log_enabled;

    if !has_logging_env {
        return;
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_env(ENV_SENTINEL_LOG)
        .or_else(|_| tracing_subscriber::EnvFilter::try_from_env(ENV_RUST_LOG))
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(DEFAULT_LOG_FILTER))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_FALLBACK_LOG_FILTER));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}

#[tokio::main]
async fn main() -> ExitCode {
    #[cfg(unix)]
    if let Err(error) = init_sigpipe() {
        eprintln!("{MAIN_ERR_SIGPIPE_INIT_FAILED}: {error}");

        return ExitCode::FAILURE;
    }

    init_logging();

    artifact_cleanup::install_cleanup_handlers();

    let cli = Cli::parse();

    if let Err(error) = artifact_store_config::init(cli.artifact_store) {
        eprintln!("{MAIN_ERR_ARTIFACT_STORE_INIT_FAILED}: {error}");

        return ExitCode::FAILURE;
    }

    match cli.command {
        Commands::Check(args) => commands::check::run(&args).await,
        Commands::Install(args) => commands::install::run_install(&args).await,
        Commands::Ci(args) => commands::install::run_ci(&args).await,
        Commands::History(args) => commands::history::run(&args).await,
    }
}
