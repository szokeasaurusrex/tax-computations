use std::path::Path;

use chrono::NaiveDate;
use corporate_actions_parser::read_csv;
use rust_decimal::Decimal;

#[test]
fn parses_effective_date_and_quantity() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corporate-actions.csv");
    let actions = read_csv(path)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(
        actions,
        vec![corporate_actions_parser::CorporateAction {
            isin: "AT0000000003".to_owned(),
            effective_date: NaiveDate::from_ymd_opt(2024, 2, 21).unwrap(),
            quantity: Decimal::from(42),
        }]
    );
}
