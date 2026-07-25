use std::{error::Error, path::PathBuf};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    about = "Calculate Austrian capital gains from cleaned trade and tax data; writes output under ./out"
)]
struct Args {
    /// Directory containing cleaned trade CSV files.
    trade_directory: PathBuf,
    /// Path to the ECB USD/EUR rates CSV.
    ecb_path: PathBuf,
    /// Path to the Meldefonds corrections CSV.
    meldefonds_path: PathBuf,
    /// Directory containing corporate-action CSV files.
    corporate_actions_directory: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    trades_parser_cli::run(
        &args.trade_directory,
        &args.ecb_path,
        &args.meldefonds_path,
        &args.corporate_actions_directory,
    )
}
