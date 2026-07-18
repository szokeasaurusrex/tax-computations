//! Types for the parser.

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct DetailedTradesRow {
    #[serde(rename = "CurrencyPrimary")]
    pub currency: Currency,
    pub symbol: String,
    #[serde(rename = "ISIN")]
    pub isin: String,
    #[serde(deserialize_with = "deserialize_ibkr_date_time")]
    pub date_time: NaiveDateTime,
    pub quantity: Decimal,
    pub proceeds: Decimal,
    pub cost_basis: Decimal,
    #[serde(rename = "Buy/Sell")]
    pub transaction_type: TransactionType,
}

/// The currency, only USD is supported now.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum Currency {
    Usd,
}

/// Whether this was a buy or sell transaction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionType {
    Buy,
    Sell,
}

/// Deserialize the IBKR date time, ignoring timezone.
fn deserialize_ibkr_date_time<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let date_time_string = String::deserialize(deserializer)?;

    NaiveDateTime::parse_from_str(&date_time_string, "%Y-%m-%d %H-%M-%S %Z")
        .map_err(D::Error::custom)
}
