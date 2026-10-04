use std::{collections::HashSet, path::Path};

use csv::{Error as CsvError, Reader};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A position held before the supplied transaction history.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct OpeningPosition {
    pub isin: String,
    pub symbol: String,
    pub quantity: Decimal,
    pub basis_eur: Decimal,
}

/// Read opening positions in the `positions.csv` output format.
///
/// A header-only file yields no positions. Other columns are ignored.
/// Quantities must be nonnegative, zero quantity requires zero basis, and
/// each ISIN may occur only once.
///
/// # Errors
///
/// Returns an error if the file cannot be opened. Deserialization, invalid
/// balance, and duplicate ISIN errors are returned by the iterator.
pub fn read_csv<P>(path: P) -> Result<impl Iterator<Item = Result<OpeningPosition, Error>>, Error>
where
    P: AsRef<Path>,
{
    let reader = Reader::from_path(path)?;

    let mut isins = HashSet::new();
    Ok(reader
        .into_deserialize::<OpeningPosition>()
        .map(move |result| {
            let position = result?;
            if position.quantity < Decimal::ZERO
                || (position.quantity == Decimal::ZERO && position.basis_eur != Decimal::ZERO)
            {
                return Err(Error::InvalidPosition(position.isin));
            }
            if !isins.insert(position.isin.clone()) {
                return Err(Error::DuplicateIsin(position.isin));
            }
            Ok(position)
        }))
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("CSV error: {0}")]
    CsvError(#[from] CsvError),
    #[error("invalid opening position for {0}: negative quantity or nonzero basis without shares")]
    InvalidPosition(String),
    #[error("duplicate opening position for {0}")]
    DuplicateIsin(String),
}
