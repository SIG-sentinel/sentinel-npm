use clap::Parser;
use sentinel::commands;
use sentinel::constants::{DEFAULT_LOG_FILTER, ENV_RUST_LOG, ENV_SENTINEL_LOG};
use sentinel::types::PrintReportSubmissionParams;
use sentinel::types::{Cli, Commands, ReportArgs};
use sentinel::ui;
use std::process::ExitCode;

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
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();
}

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::Check(args) => commands::check::run(&args).await,
        Commands::Install(args) => commands::install::run_install(&args).await,
        Commands::Ci(args) => commands::install::run_ci(&args).await,
        Commands::Report(args) => run_report(&args).await,
    }
}

async fn run_report(args: &ReportArgs) -> ExitCode {
    ui::print_report_submission(PrintReportSubmissionParams {
        package_name: &args.package,
        reason: &args.reason,
        evidence: args.evidence.as_deref(),
    });

    ExitCode::SUCCESS
}
