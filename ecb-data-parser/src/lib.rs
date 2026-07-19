use std::path::Path;

use chrono::NaiveDate;
use csv::{Error as CsvError, ReaderBuilder};
use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// An ECB reference exchange rate for a date.
#[derive(Debug, PartialEq)]
pub struct EcbEntry {
    pub date: NaiveDate,
    pub reference_rate: Decimal,
}

/// Read ECB reference rates from an ECB Data Portal CSV file.
///
/// Rows without a reference rate are omitted from the returned iterator.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or the CSV reader cannot be
/// initialized.
pub fn read_csv<P>(path: P) -> Result<impl Iterator<Item = Result<EcbEntry, Error>>, Error>
where
    P: AsRef<Path>,
{
    let reader = ReaderBuilder::new().flexible(true).from_path(path)?;

    Ok(reader
        .into_deserialize::<EcbRow>()
        .filter_map(|result| match result {
            Ok(row) => row.reference_rate.map(|reference_rate| {
                Ok(EcbEntry {
                    date: row.date,
                    reference_rate,
                })
            }),
            Err(error) => Some(Err(error.into())),
        }))
}

#[derive(Debug, Deserialize)]
struct EcbRow {
    #[serde(rename = "DATE", deserialize_with = "deserialize_date")]
    date: NaiveDate,
    #[serde(rename = "US dollar/Euro ECB reference exchange rate (EXR.D.USD.EUR.SP00.A)")]
    reference_rate: Option<Decimal>,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let date = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(D::Error::custom)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("CSV error: {0}")]
    CsvError(#[from] CsvError),
}
