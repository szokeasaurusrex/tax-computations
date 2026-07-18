use std::{error::Error, io, path::PathBuf};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Deserialize a trades CSV and display its first row")]
struct Args {
    /// Path to the CSV file.
    csv_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let rows = trades_parser::read_csv(args.csv_path)?.collect::<Result<Vec<_>, _>>()?;
    let first_row = rows
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CSV file contains no rows"))?;

    println!("{first_row:?}");

    Ok(())
}
