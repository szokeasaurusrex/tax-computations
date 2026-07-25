use std::collections::HashMap;

use crate::transaction::Transaction;

mod currency_conversion;
mod transaction;

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
