use std::path::Path;

use chrono::NaiveDate;
use csv::{Error as CsvError, ReaderBuilder};
use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// A Meldefonds correction for the taxable account.
#[derive(Debug, PartialEq)]
pub struct MeldefondsCorrection {
    pub isin: String,
    pub report_date: NaiveDate,
    pub shares_on_date_taxable: Decimal,
    pub correction_per_share_eur: Decimal,
}

/// Read Meldefonds corrections from a CSV file.
///
/// The ISIN, report date, taxable shares, and correction per share in EUR are
/// parsed. Other columns, including Roth shares, are ignored.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or a CSV row cannot be
/// deserialized.
pub fn read_csv<P>(
    path: P,
) -> Result<impl Iterator<Item = Result<MeldefondsCorrection, Error>>, Error>
where
    P: AsRef<Path>,
{
    let reader = ReaderBuilder::new().flexible(true).from_path(path)?;

    Ok(reader.into_deserialize::<MeldefondsRow>().map(|result| {
        let row = result?;
        Ok(MeldefondsCorrection {
            isin: row.isin,
            report_date: row.report_date,
            shares_on_date_taxable: row.shares_on_date_taxable,
            correction_per_share_eur: row.correction_per_share_eur,
        })
    }))
}

#[derive(Debug, Deserialize)]
struct MeldefondsRow {
    #[serde(rename = "ISIN")]
    isin: String,
    #[serde(rename = "Report date", deserialize_with = "deserialize_date")]
    report_date: NaiveDate,
    #[serde(rename = "Shares on date (taxable)")]
    shares_on_date_taxable: Decimal,
    #[serde(rename = "Correction/share (EUR)")]
    correction_per_share_eur: Decimal,
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
