use std::{path::Path, str::FromStr};

use opening_positions_parser::{Error, OpeningPosition, read_csv};
use rust_decimal::Decimal;

#[test]
fn reads_positions_and_rejects_invalid_rows() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/positions.csv");
    let mut positions = read_csv(path).unwrap();
    assert_eq!(
        positions.next().unwrap().unwrap(),
        OpeningPosition {
            isin: "AT0000000001".to_owned(),
            symbol: "AAA".to_owned(),
            quantity: Decimal::from_str("1.25").unwrap(),
            basis_eur: Decimal::from_str("123.456").unwrap(),
        },
    );
    assert_eq!(
        positions.next().unwrap().unwrap(),
        OpeningPosition {
            isin: "AT0000000002".to_owned(),
            symbol: "BBB".to_owned(),
            quantity: Decimal::ZERO,
            basis_eur: Decimal::ZERO,
        },
    );
    for expected_isin in ["AT0000000003", "AT0000000004"] {
        assert!(matches!(
            positions.next().unwrap(),
            Err(Error::InvalidPosition(isin)) if isin == expected_isin
        ));
    }
    assert!(matches!(
        positions.next().unwrap(),
        Err(Error::DuplicateIsin(isin)) if isin == "AT0000000001"
    ));
    assert!(matches!(positions.next().unwrap(), Err(Error::CsvError(_))));
    assert!(positions.next().is_none());
}
