use std::cmp::Ordering;

use chrono::{NaiveDate, NaiveDateTime};

use corporate_actions_parser::CorporateAction;
use meldefonds_corrections_parser::MeldefondsCorrection;
use serde::Serialize;

use crate::currency_conversion::TradeEur;

#[derive(Debug, PartialEq, Eq)]
enum DateTime {
    Exact(NaiveDateTime),
    EndOfDay(NaiveDate),
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => left.cmp(right),
            (Self::Exact(left), Self::EndOfDay(right)) => {
                left.date().cmp(right).then(Ordering::Less)
            }
            (Self::EndOfDay(left), Self::Exact(right)) => {
                left.cmp(&right.date()).then(Ordering::Greater)
            }
            (Self::EndOfDay(left), Self::EndOfDay(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A transaction that may change the basis.
#[derive(Debug, Serialize)]
pub(crate) enum Transaction {
    Trade(TradeEur),
    MeldefondsCorrection(MeldefondsCorrection),
    CorporateAction(CorporateAction),
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

impl From<CorporateAction> for Transaction {
    fn from(action: CorporateAction) -> Self {
        Self::CorporateAction(action)
    }
}

impl Transaction {
    pub(crate) fn isin(&self) -> &str {
        match self {
            Self::Trade(trade) => &trade.original_trade.isin,
            Self::MeldefondsCorrection(correction) => &correction.isin,
            Self::CorporateAction(action) => &action.isin,
        }
    }

    fn date_time(&self) -> DateTime {
        match self {
            Self::Trade(trade) => DateTime::Exact(trade.original_trade.date_time),
            Self::MeldefondsCorrection(correction) => DateTime::EndOfDay(correction.report_date),
            Self::CorporateAction(action) => DateTime::EndOfDay(action.effective_date),
        }
    }

    pub(crate) fn cmp_chronological(&self, other: &Self) -> Ordering {
        let ordering = self.date_time().cmp(&other.date_time());
        if ordering == Ordering::Equal
            && matches!(
                (self, other),
                (Self::MeldefondsCorrection(_), Self::CorporateAction(_))
                    | (Self::CorporateAction(_), Self::MeldefondsCorrection(_))
            )
        {
            unimplemented!(
                "corporate action and Meldefonds correction ordering on the same date is an unresolved edge case"
            );
        }
        ordering
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use corporate_actions_parser::CorporateAction;
    use meldefonds_corrections_parser::MeldefondsCorrection;
    use rust_decimal::Decimal;

    use super::Transaction;

    fn action(date: NaiveDate) -> Transaction {
        CorporateAction {
            isin: "US0000000000".to_owned(),
            effective_date: date,
            quantity: Decimal::from(42),
        }
        .into()
    }

    fn correction(date: NaiveDate) -> Transaction {
        MeldefondsCorrection {
            isin: "US0000000000".to_owned(),
            report_date: date,
            shares_on_date_taxable: Decimal::from(42),
            correction_per_share_eur: Decimal::ZERO,
        }
        .into()
    }

    #[test]
    fn corporate_action_is_after_events_on_its_date() {
        let action = action(NaiveDate::from_ymd_opt(2024, 2, 21).unwrap());
        let correction = correction(NaiveDate::from_ymd_opt(2024, 2, 22).unwrap());

        assert_eq!(
            action.cmp_chronological(&correction),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    #[should_panic(expected = "corporate action and Meldefonds correction ordering")]
    fn same_day_corporate_action_and_correction_are_unimplemented() {
        let action = action(NaiveDate::from_ymd_opt(2024, 2, 21).unwrap());
        let correction = correction(NaiveDate::from_ymd_opt(2024, 2, 21).unwrap());

        action.cmp_chronological(&correction);
    }
}
