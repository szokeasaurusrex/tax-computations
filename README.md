# Austrian capital-gains computation

I built this Rust CLI to help prepare my Austrian tax return from my brokerage data. It is very tailored to my setup: one depot, cleaned Interactive Brokers-style CSV exports, USD trades, ECB exchange rates, Meldefonds corrections, and selected quantity-changing corporate actions.

> [!WARNING]
> This project is not tax advice or tax-return software. It does not establish tax compliance or replace advice from a qualified Austrian tax professional. Verify the rules, input data, and results independently before using them for a filing or financial decision.

## What it does

The CLI processes events chronologically, separately for each ISIN, and produces an auditable moving-average basis and realized-gain ledger.

- Purchases increase quantity and EUR basis.
- Sales remove proportional moving-average basis and calculate realized gain or loss.
- Meldefonds corrections change basis without changing quantity.
- Corporate actions change quantity without changing basis.

The implementation is experimental and intentionally narrow. It is not a general-purpose tax calculator.

## Usage

The CLI takes four positional paths:

```text
cargo run -- <trade-directory> <ecb-rates.csv> <meldefonds-corrections.csv> <corporate-actions-directory>
```

Example:

```text
cargo run -- \
  data/statements \
  data/ecb-rates.csv \
  data/meldefonds-corrections.csv \
  data/corporate_actions
```

It writes:

- `out/events.csv`: chronological event ledger;
- `out/positions.csv`: final quantity and EUR basis per ISIN.

The `out/` directory is ignored by Git.

## Inputs

The trade directory contains cleaned CSV files with `CurrencyPrimary`, `Symbol`, `ISIN`, `DateTime`, `Quantity`, `Proceeds`, and `Buy/Sell`. Only CSV files directly inside the directory are read. The current implementation supports USD trades. Buys use positive quantity and negative proceeds; sales use negative quantity and positive proceeds.

The ECB CSV provides USD/EUR rates. If a rate is missing for a transaction date, the CLI searches preceding dates within seven days.

The Meldefonds CSV provides `ISIN`, `Report date`, `Shares on date (taxable)`, and `Correction/share (EUR)`. The reported quantity must match the calculated position within `0.01` units.

Corporate-action CSVs provide `ISIN`, `Date/Time` in `YYYY-MM-DD` format, and `Quantity`. Other brokerage columns, including `Report Date`, are ignored. Actions occur at the end of their effective date. A zero-position action warns but still applies its quantity. An action and Meldefonds correction on the same date are currently an unresolved edge case and cause an explicit panic.

## Output

The first column of `out/events.csv` is `Transaction type`, with values `Trade`, `Meldefonds Correction`, or `Corporate Action`. The ledger includes source values, quantities, proceeds, EUR proceeds, total quantity, total EUR basis, and realized gain where applicable.

`out/positions.csv` contains the final ISIN, symbol, quantity, and EUR basis. These files are calculation output, not tax forms.

## Development

```text
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-features
cargo check --workspace --all-targets --all-features
```

The main components are [`trades-parser`](trades-parser), [`ecb-data-parser`](ecb-data-parser), [`meldefonds-corrections-parser`](meldefonds-corrections-parser), [`corporate-actions-parser`](corporate-actions-parser), and [`trades-parser-cli`](trades-parser-cli).

## Limitations

The implementation currently assumes one depot, cleaned input, USD trades, and known acquisition history. It does not handle transferred-in basis, gifts, inheritances, depot transfers, short positions, general corporate-action types, deduplication, tax-form rounding, or tax-return generation.
