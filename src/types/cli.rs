use clap::{Parser, Subcommand};

use crate::constants::CLI_LONG_ABOUT;
use crate::types::{ArtifactStore, CheckArgs, CiArgs, HistoryArgs, InstallArgs};

#[derive(Parser, Debug)]
#[command(
    name = "sentinel",
    version = env!("CARGO_PKG_VERSION"),
    about = "Supply chain security for npm",
    long_about = CLI_LONG_ABOUT
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        value_name = "memory|spool|auto"
    )]
    pub artifact_store: ArtifactStore,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Audit dependencies without installing packages")]
    Check(CheckArgs),
    #[command(about = "Install one or more packages with atomic verification")]
    Install(InstallArgs),
    #[command(about = "Verify full lockfile then run clean install")]
    Ci(CiArgs),
    #[command(about = "Query local install/ci history ledger")]
    History(HistoryArgs),
}
