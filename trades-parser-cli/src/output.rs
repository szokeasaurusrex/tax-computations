use std::{fs, io};

use serde::Serialize;
use thiserror::Error;

use crate::calculation::CalculatedTransaction;

#[derive(Debug, Error)]
pub(crate) enum OutputError {
    #[error("failed to create output directory: {0}")]
    Io(#[from] io::Error),
    #[error("failed to serialize output: {0}")]
    Csv(#[from] csv::Error),
}

#[derive(Serialize)]
pub(crate) struct PositionRow {
    pub(crate) isin: String,
    pub(crate) symbol: String,
    pub(crate) quantity: rust_decimal::Decimal,
    pub(crate) basis_eur: rust_decimal::Decimal,
}

pub(crate) fn write_outputs(
    events: &[CalculatedTransaction],
    positions: &[PositionRow],
) -> Result<(), OutputError> {
    fs::create_dir_all("out")?;

    let mut events_writer = csv::Writer::from_path("out/events.csv")?;
    for event in events {
        events_writer.serialize(event)?;
    }
    events_writer.flush()?;

    let mut positions_writer = csv::Writer::from_path("out/positions.csv")?;
    for position in positions {
        positions_writer.serialize(position)?;
    }
    positions_writer.flush()?;

    Ok(())
}
