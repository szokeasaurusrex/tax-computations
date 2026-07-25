use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use ecb_data_parser::EcbEntry;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;
use trades_parser::DetailedTradesRow;

#[derive(Debug, Serialize)]
pub(crate) struct TradeEur {
    #[serde(flatten)]
    pub original_trade: DetailedTradesRow,
    #[serde(rename = "Proceeds (EUR)")]
    pub proceeds_eur: Decimal,
}

impl TradeEur {
    pub(crate) fn try_from_row(
        row: DetailedTradesRow,
        conversions: &RateTable,
    ) -> Result<Self, RateNotFound> {
        let &DetailedTradesRow {
            proceeds: proceeds_usd,
            date_time,
            ..
        } = &row;

        let proceeds_eur = conversions.usd_to_eur(proceeds_usd, date_time.date())?;

        Ok(Self {
            original_trade: row,
            proceeds_eur,
        })
    }
}

/// A lookup table of ECB exchange rates by date.
///
/// The ECB rates are in units of USD/EUR.
pub(crate) struct RateTable(HashMap<NaiveDate, Decimal>);

#[derive(Debug, Error)]
#[error("no exchange rate found for the date or the preceding seven days")]
pub(crate) struct RateNotFound;

impl RateTable {
    /// Build a rate table from an iterator of ECB rates.
    ///
    /// Errors are propagated out.
    pub(crate) fn try_from_rates<I, E>(rates: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<EcbEntry, E>>,
    {
        rates
            .into_iter()
            .map(|result| {
                result.map(
                    |EcbEntry {
                         date,
                         reference_rate,
                     }| (date, reference_rate),
                )
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    /// Convert a USD amount to EUR using the date's exchange rate.
    ///
    /// Returns the EUR amount or an error if no corresponding rate is found.
    fn usd_to_eur(&self, usd_amount: Decimal, date: NaiveDate) -> Result<Decimal, RateNotFound> {
        Ok(usd_amount / self.get_rate(date)?)
    }

    /// Get the rate for `date`, searching up to seven preceding days when necessary.
    ///
    /// # Errors
    ///
    /// Returns [`RateNotFound`] when no rate is available within the seven preceding
    /// days.
    fn get_rate(&self, date: NaiveDate) -> Result<Decimal, RateNotFound> {
        (0..=7)
            .find_map(|days| self.0.get(&(date - Duration::days(days))))
            .copied()
            .ok_or(RateNotFound)
    }
}
