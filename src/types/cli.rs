use clap::{Parser, Subcommand};

use crate::constants::CLI_LONG_ABOUT;
use crate::types::{CheckArgs, CiArgs, InstallArgs, ReportArgs};

#[derive(Parser, Debug)]
#[command(
    name = "sentinel",
    version = env!("CARGO_PKG_VERSION"),
    about = "Supply chain security for npm",
    long_about = CLI_LONG_ABOUT
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Check(CheckArgs),
    Install(InstallArgs),
    Ci(CiArgs),
    Report(ReportArgs),
}
