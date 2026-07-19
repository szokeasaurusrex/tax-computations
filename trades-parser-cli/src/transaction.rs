use meldefonds_corrections_parser::MeldefondsCorrection;
use trades_parser::DetailedTradesRow;

/// A transaction that may change the basis.
pub(crate) enum Transaction {
    Trade(DetailedTradesRow),
    MeldefondsCorrection(MeldefondsCorrection),
}
