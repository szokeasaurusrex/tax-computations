# Austrian capital-gains computation

I built this Rust CLI to help prepare my Austrian tax return from my brokerage data. It is very tailored to my setup: one depot, cleaned Interactive Brokers-style CSV exports, USD trades, ECB exchange rates, Meldefonds corrections, and selected quantity-changing corporate actions.

> [!WARNING]
> This project is not tax advice or tax-return software. It does not establish tax compliance or replace advice from a qualified Austrian tax professional. Verify the rules, input data, and results independently before using them for a filing or financial decision.

## What it does

The CLI processes events chronologically, separately for each ISIN, and produces an auditable moving-average basis and realized-gain ledger.

- Opening positions initialize quantity and EUR basis before dated events.
- Purchases increase quantity and EUR basis.
- Sales remove proportional moving-average basis and calculate realized gain or loss.
- Meldefonds corrections change basis without changing quantity.
- Corporate actions change quantity without changing basis.

The implementation is experimental and intentionally narrow. It is not a general-purpose tax calculator.

## Usage

The CLI takes five required positional paths, starting with the opening positions CSV:

```text
cargo run -- <opening-positions.csv> <trade-directory> <ecb-rates.csv> <meldefonds-corrections.csv> <corporate-actions-directory>
```

Example:

```text
cargo run -- \
  data/opening-positions.csv \
  data/statements \
  data/ecb-rates.csv \
  data/meldefonds-corrections.csv \
  data/corporate_actions
```

It writes:

- `out/events.csv`: chronological event ledger;
- `out/positions.csv`: final quantity and EUR basis per ISIN.

The `out/` directory is ignored by Git.

### Continue from a previous year

Use the previous year's `out/positions.csv` unchanged as the first argument:

```text
cargo run -- \
  data/previous-year/positions.csv \
  data/current-year/trades \
  data/ecb-rates.csv \
  data/current-year/meldefonds-corrections.csv \
  data/current-year/corporate_actions
```

Opening balances use the columns `isin,symbol,quantity,basis_eur`. Each ISIN may occur only once. Quantities must be nonnegative, and zero quantity requires zero basis. Closed positions from previous output are accepted. To start from zero, supply a file containing only this header:

```csv
isin,symbol,quantity,basis_eur
```

Opening positions are ledger events ordered before all dated events. No date filtering is performed: include only trades, corrections, and corporate actions after the opening snapshot to avoid double-counting. ISINs missing from the opening file start at zero. Positions without new activity remain in the position output, with their original symbols; a new trade supplies the latest symbol.

Save the previous output as a separate input file: every run overwrites `out/positions.csv` and `out/events.csv`.

## Inputs

The trade directory contains cleaned CSV files with `CurrencyPrimary`, `Symbol`, `ISIN`, `DateTime`, `Quantity`, `Proceeds`, and `Buy/Sell`. Only CSV files directly inside the directory are read. The current implementation supports USD trades. Buys use positive quantity and negative proceeds; sales use negative quantity and positive proceeds.

The ECB CSV provides USD/EUR rates. If a rate is missing for a transaction date, the CLI searches preceding dates within seven days.

The Meldefonds CSV provides `ISIN`, `Report date`, `Shares on date (taxable)`, and `Correction/share (EUR)`. The reported quantity must match the calculated position within `0.01` units.

Corporate-action CSVs provide `ISIN`, `Date/Time` in `YYYY-MM-DD` format, and `Quantity`. Other brokerage columns, including `Report Date`, are ignored. Actions occur at the end of their effective date. A zero-position action warns but still applies its quantity. An action and Meldefonds correction on the same date are currently an unresolved edge case and cause an explicit panic.

## Output

The first column of `out/events.csv` is `Transaction type`, with values `Opening Position`, `Trade`, `Meldefonds Correction`, or `Corporate Action`. The ledger includes source values, quantities, proceeds, EUR proceeds, total quantity, total EUR basis, and realized gain where applicable.

Opening rows retain the ISIN, symbol, and quantity; `total_basis_eur` contains the supplied opening basis. Dates, currency, proceeds, buy/sell, and realized gain are empty. Opening positions are not purchases and do not require exchange-rate conversion. The CSV columns are unchanged; `Opening Position` is an additional event type.

`out/positions.csv` contains the final ISIN, symbol, quantity, and EUR basis. These files are calculation output, not tax forms.

## Development

```text
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-features
cargo check --workspace --all-targets --all-features
```

The input parsers are [`trades-parser`](trades-parser), [`ecb-data-parser`](ecb-data-parser), [`meldefonds-corrections-parser`](meldefonds-corrections-parser), [`corporate-actions-parser`](corporate-actions-parser), and [`opening-positions-parser`](opening-positions-parser). Each exposes input values through `read_csv`. The opening-positions parser also validates balances and rejects duplicate ISINs. The [`trades-parser-cli`](trades-parser-cli) crate applies transaction validation, calculates one ISIN at a time, and serializes output separately.

## Limitations

The implementation currently assumes one depot, cleaned input, USD trades, and known acquisition history. It does not handle transferred-in basis, gifts, inheritances, depot transfers, short positions, general corporate-action types, deduplication, tax-form rounding, or tax-return generation.
