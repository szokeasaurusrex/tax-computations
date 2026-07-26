use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Serialize, Serializer};
use thiserror::Error;
use trades_parser::TransactionType;

use super::Transaction;

#[derive(Debug)]
pub(crate) struct CalculatedTransaction {
    /// The transaction event.
    pub(crate) transaction: Transaction,
    /// Remaining quantity after transaction.
    pub(crate) total_quantity: Decimal,
    /// Remaining cost basis after transaction.
    pub(crate) total_basis_eur: Decimal,
    /// For sales only: net realized gain in Euros.
    pub(crate) net_gain_eur: Option<Decimal>,
}

impl Serialize for CalculatedTransaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Row<'a> {
            #[serde(rename = "Transaction type")]
            event_type: &'static str,
            #[serde(rename = "CurrencyPrimary")]
            currency: Option<&'a trades_parser::Currency>,
            symbol: Option<&'a str>,
            #[serde(rename = "ISIN")]
            isin: &'a str,
            #[serde(rename = "DateTime")]
            date_time: Option<&'a NaiveDateTime>,
            quantity: Option<Decimal>,
            proceeds: Option<Decimal>,
            #[serde(rename = "Buy/Sell")]
            transaction_type: Option<&'a TransactionType>,
            #[serde(rename = "Proceeds (EUR)")]
            proceeds_eur: Option<Decimal>,
            #[serde(rename = "Report date")]
            report_date: Option<NaiveDate>,
            #[serde(rename = "Corporate action date")]
            corporate_action_date: Option<NaiveDate>,
            #[serde(rename = "Shares on date (taxable)")]
            correction_shares: Option<Decimal>,
            #[serde(rename = "Correction/share (EUR)")]
            correction_per_share_eur: Option<Decimal>,
            total_quantity: Decimal,
            total_basis_eur: Decimal,
            net_gain_eur: Option<Decimal>,
        }

        match &self.transaction {
            Transaction::Trade(trade) => Row {
                event_type: "Trade",
                currency: Some(&trade.original_trade.currency),
                symbol: Some(&trade.original_trade.symbol),
                isin: &trade.original_trade.isin,
                date_time: Some(&trade.original_trade.date_time),
                quantity: Some(trade.original_trade.quantity),
                proceeds: Some(trade.original_trade.proceeds),
                transaction_type: Some(&trade.original_trade.transaction_type),
                proceeds_eur: Some(trade.proceeds_eur),
                report_date: None,
                corporate_action_date: None,
                correction_shares: None,
                correction_per_share_eur: None,
                total_quantity: self.total_quantity,
                total_basis_eur: self.total_basis_eur,
                net_gain_eur: self.net_gain_eur,
            }
            .serialize(serializer),
            Transaction::MeldefondsCorrection(correction) => Row {
                event_type: "Meldefonds Correction",
                currency: None,
                symbol: None,
                isin: &correction.isin,
                date_time: None,
                quantity: None,
                proceeds: None,
                transaction_type: None,
                proceeds_eur: None,
                report_date: Some(correction.report_date),
                corporate_action_date: None,
                correction_shares: Some(correction.shares_on_date_taxable),
                correction_per_share_eur: Some(correction.correction_per_share_eur),
                total_quantity: self.total_quantity,
                total_basis_eur: self.total_basis_eur,
                net_gain_eur: self.net_gain_eur,
            }
            .serialize(serializer),
            Transaction::CorporateAction(action) => Row {
                event_type: "Corporate Action",
                currency: None,
                symbol: None,
                isin: &action.isin,
                date_time: None,
                quantity: Some(action.quantity),
                proceeds: None,
                transaction_type: None,
                proceeds_eur: None,
                report_date: None,
                corporate_action_date: Some(action.effective_date),
                correction_shares: None,
                correction_per_share_eur: None,
                total_quantity: self.total_quantity,
                total_basis_eur: self.total_basis_eur,
                net_gain_eur: None,
            }
            .serialize(serializer),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum CalculationError {
    #[error("correction has no position")]
    CorrectionWithoutPosition,
    #[error(
        "correction quantity mismatch for {isin} on {report_date}: expected position quantity {expected}, actual correction quantity {actual}"
    )]
    CorrectionQuantityMismatch {
        isin: String,
        report_date: NaiveDate,
        expected: Decimal,
        actual: Decimal,
    },
}

pub(crate) fn calculate(
    transactions: Vec<Transaction>,
) -> Result<Vec<CalculatedTransaction>, CalculationError> {
    let mut total_quantity = Decimal::ZERO;
    let mut total_basis_eur = Decimal::ZERO;
    transactions
        .into_iter()
        .map(|transaction| {
            let net_gain_eur = match &transaction {
                Transaction::Trade(trade) => match &trade.original_trade.transaction_type {
                    TransactionType::Buy => {
                        total_quantity += trade.original_trade.quantity;
                        total_basis_eur -= trade.proceeds_eur;
                        None
                    }
                    TransactionType::Sell => {
                        if total_quantity == Decimal::ZERO {
                            None
                        } else {
                            let quantity_sold = -trade.original_trade.quantity;
                            let basis_disposed = quantity_sold * (total_basis_eur / total_quantity);
                            total_quantity -= quantity_sold;
                            total_basis_eur -= basis_disposed;
                            if total_quantity == Decimal::ZERO {
                                total_basis_eur = Decimal::ZERO;
                            }
                            Some(trade.proceeds_eur - basis_disposed)
                        }
                    }
                },
                Transaction::MeldefondsCorrection(correction) => {
                    if total_quantity == Decimal::ZERO {
                        return Err(CalculationError::CorrectionWithoutPosition);
                    }
                    if (total_quantity - correction.shares_on_date_taxable).abs()
                        > Decimal::new(1, 2)
                    {
                        return Err(CalculationError::CorrectionQuantityMismatch {
                            isin: correction.isin.clone(),
                            report_date: correction.report_date,
                            expected: total_quantity,
                            actual: correction.shares_on_date_taxable,
                        });
                    }

                    total_basis_eur +=
                        correction.shares_on_date_taxable * correction.correction_per_share_eur;
                    None
                }
                Transaction::CorporateAction(action) => {
                    if total_quantity == Decimal::ZERO {
                        eprintln!(
                            "warning: corporate action for {} on {} has no position; applying quantity change of {}",
                            action.isin, action.effective_date, action.quantity
                        );
                    }
                    total_quantity += action.quantity;
                    None
                }
            };

            Ok(CalculatedTransaction {
                transaction,
                total_quantity,
                total_basis_eur,
                net_gain_eur,
            })
        })
        .collect()
}
