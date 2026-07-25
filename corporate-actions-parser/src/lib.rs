use std::path::Path;

use chrono::NaiveDate;
use csv::{Error as CsvError, ReaderBuilder};
use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// A corporate action that changes the number of held shares.
#[derive(Debug, PartialEq)]
pub struct CorporateAction {
    pub isin: String,
    pub effective_date: NaiveDate,
    pub quantity: Decimal,
}

/// Read corporate actions from an Interactive Brokers CSV file.
///
/// The ISIN, effective date from `Date/Time`, and quantity are parsed. Other
/// columns, including `Report Date`, are ignored.
///
/// # Errors
///
/// Returns an error if the CSV file cannot be opened or a row cannot be
/// deserialized.
pub fn read_csv<P>(path: P) -> Result<impl Iterator<Item = Result<CorporateAction, Error>>, Error>
where
    P: AsRef<Path>,
{
    let reader = ReaderBuilder::new().flexible(true).from_path(path)?;

    Ok(reader
        .into_deserialize::<CorporateActionRow>()
        .map(|result| {
            let row = result?;
            Ok(CorporateAction {
                isin: row.isin,
                effective_date: row.effective_date,
                quantity: row.quantity,
            })
        }))
}

#[derive(Debug, Deserialize)]
struct CorporateActionRow {
    #[serde(rename = "ISIN")]
    isin: String,
    #[serde(rename = "Date/Time", deserialize_with = "deserialize_date")]
    effective_date: NaiveDate,
    #[serde(rename = "Quantity")]
    quantity: Decimal,
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
