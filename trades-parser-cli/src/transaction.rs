use meldefonds_corrections_parser::MeldefondsCorrection;
use trades_parser::DetailedTradesRow;

use crate::currency_conversion::TradeEur;

/// A transaction that may change the basis.
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
    pub fn isin(&self) -> &str {
        match self {
            Self::Trade(trade) => &trade.original_trade.isin,
            Self::MeldefondsCorrection(correction) => &correction.isin,
        }
    }
}
