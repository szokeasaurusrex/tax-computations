use std::cmp::Ordering;

use meldefonds_corrections_parser::MeldefondsCorrection;
use serde::Serialize;

use crate::currency_conversion::TradeEur;

/// A transaction that may change the basis.
#[derive(Debug, Serialize)]
pub(crate) enum Transaction {
    Trade(TradeEur),
    MeldefondsCorrection(MeldefondsCorrection),
}

impl From<TradeEur> for Transaction {
    fn from(trade: TradeEur) -> Self {
        Self::Trade(trade)
    }
}

impl From<MeldefondsCorrection> for Transaction {
    fn from(correction: MeldefondsCorrection) -> Self {
        Self::MeldefondsCorrection(correction)
    }
}

impl Transaction {
    pub(crate) fn isin(&self) -> &str {
        match self {
            Self::Trade(trade) => &trade.original_trade.isin,
            Self::MeldefondsCorrection(correction) => &correction.isin,
        }
    }

    pub(crate) fn cmp_chronological(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Trade(left), Self::Trade(right)) => left
                .original_trade
                .date_time
                .cmp(&right.original_trade.date_time),
            (Self::MeldefondsCorrection(left), Self::MeldefondsCorrection(right)) => {
                left.report_date.cmp(&right.report_date)
            }
            (Self::Trade(trade), Self::MeldefondsCorrection(correction)) => trade
                .original_trade
                .date_time
                .date()
                .cmp(&correction.report_date)
                .then(Ordering::Less),
            (Self::MeldefondsCorrection(correction), Self::Trade(trade)) => correction
                .report_date
                .cmp(&trade.original_trade.date_time.date())
                .then(Ordering::Greater),
        }
    }
}
