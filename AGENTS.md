# Project guidelines

- Keep the Rust code short, direct, and easy to verify.
- Use `rust_decimal::Decimal` for financial quantities and calculations. Do not add implicit rounding.
- Preserve source transactions in calculated rows and keep output serialization explicit.
- Process one ISIN at a time after `split_by_isin`.
- Keep parser crates focused on their input format; do not mix parsing with calculation.
- Prefer fabricated fixtures. Never commit real brokerage statements, account identifiers, personal data, generated output, or secrets.
- Keep tax-rule descriptions high-level and avoid presenting implementation assumptions as legal conclusions.
- Preserve the existing CLI input and output contracts unless a change is intentional and documented.
- Run these checks after changes:

  ```text
  cargo fmt --check
  cargo test --workspace
  cargo clippy --workspace --all-features
  cargo check --workspace --all-targets --all-features
  ```
