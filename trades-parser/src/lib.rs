mod types;

use std::path::Path;

use csv::{Error as CsvError, Reader};
use thiserror::Error;

pub use self::types::{Currency, DetailedTradesRow, TransactionType};

pub fn read_csv<P>(path: P) -> Result<impl Iterator<Item = Result<DetailedTradesRow, Error>>, Error>
where
    P: AsRef<Path>,
{
    let rdr = Reader::from_path(path)?;

    Ok(rdr.into_deserialize().map(|result| Ok(result?)))
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("CSV error: {0}")]
    CsvError(#[from] CsvError),
}
