use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;
use trades_parser::TransactionType;

use super::Transaction;

#[derive(Serialize)]
pub(crate) struct CalculatedTransaction {
    /// The transaction event.
    #[serde(flatten)]
    pub(crate) transaction: Transaction,
    /// Remaining quantity after transaction.
    pub(crate) total_quantity: Decimal,
    /// Remaining cost basis after transaction.
    pub(crate) total_basis_eur: Decimal,
    /// For sales only: net realized gain in Euros.
    pub(crate) net_gain_eur: Option<Decimal>,
}

#[derive(Debug, Error)]
pub(crate) enum CalculationError {
    #[error("correction has no position")]
    CorrectionWithoutPosition,
    #[error("correction quantity does not match position")]
    CorrectionQuantityMismatch,
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
                        return Err(CalculationError::CorrectionQuantityMismatch);
                    }

                    total_basis_eur +=
                        correction.shares_on_date_taxable * correction.correction_per_share_eur;
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
