use std::{collections::HashMap, fs, path::Path};

use thiserror::Error;
use trades_parser::TransactionType;

use crate::{
    calculation::calculate,
    currency_conversion::{RateNotFound, RateTable, TradeEur},
    output::{PositionRow, write_outputs},
    transaction::Transaction,
};

#[derive(Debug, Error)]
pub(crate) enum InputError {
    #[error("failed to read trade directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to read ECB rates: {0}")]
    Ecb(#[from] ecb_data_parser::Error),
    #[error("failed to read trades: {0}")]
    Trades(#[from] trades_parser::Error),
    #[error("failed to read Meldefonds corrections: {0}")]
    Meldefonds(#[from] meldefonds_corrections_parser::Error),
    #[error("failed to read corporate actions: {0}")]
    CorporateActions(#[from] corporate_actions_parser::Error),
    #[error("exchange rate not found: {0}")]
    RateNotFound(#[from] RateNotFound),
    #[error("invalid quantity or proceeds signs for {isin} {symbol}")]
    InvalidTradeSigns { isin: String, symbol: String },
    #[error("failed to read opening positions: {0}")]
    OpeningPositions(#[from] opening_positions_parser::Error),
}

fn read_trade_file(path: &Path, rates: &RateTable) -> Result<Vec<Transaction>, InputError> {
    trades_parser::read_csv(path)?
        .map(|row| {
            let row = row?;
            let valid_signs = match &row.transaction_type {
                TransactionType::Buy => {
                    row.quantity > rust_decimal::Decimal::ZERO
                        && row.proceeds < rust_decimal::Decimal::ZERO
                }
                TransactionType::Sell => {
                    row.quantity < rust_decimal::Decimal::ZERO
                        && row.proceeds > rust_decimal::Decimal::ZERO
                }
            };
            if !valid_signs {
                return Err(InputError::InvalidTradeSigns {
                    isin: row.isin,
                    symbol: row.symbol,
                });
            }

            Ok(TradeEur::try_from_row(row, rates)?.into())
        })
        .collect()
}

/// Read, convert, validate, and chronologically order all input transactions.
pub(crate) fn load_transactions(
    opening_positions_path: &Path,
    trade_directory: &Path,
    ecb_path: &Path,
    meldefonds_path: &Path,
    corporate_actions_directory: &Path,
) -> Result<Vec<Transaction>, InputError> {
    let rates = RateTable::try_from_rates(ecb_data_parser::read_csv(ecb_path)?)?;
    let mut transactions = fs::read_dir(trade_directory)?
        .map(|entry| {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "csv") {
                return Ok(Vec::new());
            }

            read_trade_file(&path, &rates)
        })
        .try_fold(Vec::new(), |mut transactions, file| {
            transactions.extend(file?);
            Ok::<_, InputError>(transactions)
        })?;

    transactions.extend(
        opening_positions_parser::read_csv(opening_positions_path)?
            .map(|position| position.map(Transaction::from))
            .collect::<Result<Vec<_>, _>>()?,
    );
    transactions.extend(
        meldefonds_corrections_parser::read_csv(meldefonds_path)?
            .map(|correction| correction.map(Transaction::from))
            .collect::<Result<Vec<_>, _>>()?,
    );

    for entry in fs::read_dir(corporate_actions_directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "csv") {
            continue;
        }
        transactions.extend(
            corporate_actions_parser::read_csv(path.clone())?
                .map(|action| action.map(Transaction::from))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }

    transactions.sort_by(Transaction::cmp_chronological);
    Ok(transactions)
}

mod calculation;
mod currency_conversion;
mod output;
mod transaction;

/// Run from opening positions and write both output files.
///
/// Opening balances apply before all supplied events; no date filtering is performed.
///
/// # Errors
///
/// Returns an error when input loading, calculation, or output writing fails.
pub fn run(
    opening_positions_path: &Path,
    trade_directory: &Path,
    ecb_path: &Path,
    meldefonds_path: &Path,
    corporate_actions_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let transactions = load_transactions(
        opening_positions_path,
        trade_directory,
        ecb_path,
        meldefonds_path,
        corporate_actions_directory,
    )?;
    let groups = split_by_isin(transactions)
        .into_iter()
        .map(
            |(isin, transactions)| -> Result<_, Box<dyn std::error::Error>> {
                let calculated = calculate(transactions)?;
                let (quantity, basis_eur) = calculated
                    .last()
                    .map(|event| (event.total_quantity, event.total_basis_eur))
                    .ok_or_else(|| std::io::Error::other("ISIN has no transactions"))?;
                let symbol = calculated
                    .iter()
                    .rev()
                    .find_map(|event| match &event.transaction {
                        Transaction::OpeningPosition(position) => Some(position.symbol.clone()),
                        Transaction::Trade(trade) => Some(trade.original_trade.symbol.clone()),
                        Transaction::MeldefondsCorrection(_) | Transaction::CorporateAction(_) => {
                            None
                        }
                    });

                Ok((
                    calculated,
                    PositionRow {
                        isin,
                        symbol: symbol.unwrap_or_default(),
                        quantity,
                        basis_eur,
                    },
                ))
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let (event_groups, positions): (Vec<_>, Vec<_>) = groups.into_iter().unzip();
    let mut events = event_groups.into_iter().flatten().collect::<Vec<_>>();

    events.sort_by(|left, right| left.transaction.cmp_chronological(&right.transaction));
    write_outputs(&events, &positions)?;
    Ok(())
}

/// Split a list of transactions into a map keyed by their ISIN.
///
/// The ordering of the transactions is preserved.
fn split_by_isin<I>(transactions: I) -> HashMap<String, Vec<Transaction>>
where
    I: IntoIterator<Item = Transaction>,
{
    transactions
        .into_iter()
        .fold(HashMap::new(), |mut map, transaction| {
            let isin = transaction.isin();
            map.entry(isin.to_owned()).or_default().push(transaction);
            map
        })
}
