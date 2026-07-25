use std::cmp::Ordering;

use chrono::NaiveDate;
use meldefonds_corrections_parser::MeldefondsCorrection;
use serde::Serialize;
use trades_parser::TransactionType;

use crate::currency_conversion::TradeEur;

/// The kind of event represented by a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventKind {
    Buy,
    Sale,
    MeldefondsCorrection,
}

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

    pub(crate) fn date(&self) -> NaiveDate {
        match self {
            Self::Trade(trade) => trade.original_trade.date_time.date(),
            Self::MeldefondsCorrection(correction) => correction.report_date,
        }
    }

    pub(crate) fn event_kind(&self) -> EventKind {
        match self {
            Self::Trade(trade) => match trade.original_trade.transaction_type {
                TransactionType::Buy => EventKind::Buy,
                TransactionType::Sell => EventKind::Sale,
            },
            Self::MeldefondsCorrection(_) => EventKind::MeldefondsCorrection,
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
