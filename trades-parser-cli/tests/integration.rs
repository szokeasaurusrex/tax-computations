use rust_decimal::Decimal;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_directory(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tax-computations-{name}-{suffix}"));
    fs::create_dir_all(&path).expect("create temporary directory");
    path
}

fn write_inputs(root: &Path, corrections: &str) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let trades = root.join("trades");
    fs::create_dir_all(&trades).expect("create trade directory");
    fs::copy(fixtures.join("fake-trades.csv"), trades.join("fake-trades.csv"))
        .expect("copy fake trades");
    fs::copy(fixtures.join("ecb.csv"), root.join("ecb.csv")).expect("copy fake ECB rates");
    fs::copy(fixtures.join(corrections), root.join("corrections.csv"))
        .expect("copy fake corrections");
}

fn run(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_trades-parser-cli"))
        .args([
            root.join("trades"),
            root.join("ecb.csv"),
            root.join("corrections.csv"),
        ])
        .current_dir(root)
        .output()
        .expect("run CLI")
}

fn value<'a>(record: &'a csv::StringRecord, headers: &csv::StringRecord, name: &str) -> &'a str {
    record
        .get(headers.iter().position(|header| header == name).unwrap())
        .unwrap()
}

fn decimal(record: &csv::StringRecord, headers: &csv::StringRecord, name: &str) -> Decimal {
    Decimal::from_str(value(record, headers, name)).unwrap()
}

fn optional_decimal(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
    name: &str,
) -> Option<Decimal> {
    match value(record, headers, name) {
        "" => None,
        value => Some(Decimal::from_str(value).unwrap()),
    }
}

struct EventExpectation<'a> {
    isin: &'a str,
    symbol: &'a str,
    quantity: Option<&'a str>,
    proceeds: Option<&'a str>,
    proceeds_eur: Option<&'a str>,
    total_quantity: &'a str,
    total_basis_eur: &'a str,
    net_gain_eur: Option<&'a str>,
}

fn assert_event(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
    expected: &EventExpectation<'_>,
) {
    assert_eq!(value(record, headers, "ISIN"), expected.isin);
    assert_eq!(value(record, headers, "symbol"), expected.symbol);
    assert_eq!(
        optional_decimal(record, headers, "quantity"),
        expected.quantity.map(|value| Decimal::from_str(value).unwrap())
    );
    assert_eq!(
        optional_decimal(record, headers, "proceeds"),
        expected.proceeds.map(|value| Decimal::from_str(value).unwrap())
    );
    assert_eq!(
        optional_decimal(record, headers, "Proceeds (EUR)"),
        expected
            .proceeds_eur
            .map(|value| Decimal::from_str(value).unwrap())
    );
    assert_eq!(
        decimal(record, headers, "total_quantity"),
        Decimal::from_str(expected.total_quantity).unwrap()
    );
    assert_eq!(
        decimal(record, headers, "total_basis_eur"),
        Decimal::from_str(expected.total_basis_eur).unwrap()
    );
    assert_eq!(
        optional_decimal(record, headers, "net_gain_eur"),
        expected
            .net_gain_eur
            .map(|value| Decimal::from_str(value).unwrap())
    );
}

#[test]
fn calculates_fake_trades_and_weekend_rate() {
    let root = temp_directory("success");
    write_inputs(&root, "corrections-valid.csv");
    let output = run(&root);
    assert!(output.status.success(), "CLI failed: {output:?}");

    let events = fs::read_to_string(root.join("out/events.csv")).expect("read events");
    let mut reader = csv::Reader::from_reader(events.as_bytes());
    let headers = reader.headers().unwrap().clone();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    let expected = [
        EventExpectation { isin: "AT0000000001", symbol: "AAA", quantity: Some("10"), proceeds: Some("-1000"), proceeds_eur: Some("-500"), total_quantity: "10", total_basis_eur: "500", net_gain_eur: None },
        EventExpectation { isin: "AT0000000001", symbol: "AAA", quantity: Some("5"), proceeds: Some("-600"), proceeds_eur: Some("-200"), total_quantity: "15", total_basis_eur: "700", net_gain_eur: None },
        EventExpectation { isin: "AT0000000001", symbol: "AAA", quantity: Some("-6"), proceeds: Some("720"), proceeds_eur: Some("180"), total_quantity: "9", total_basis_eur: "420", net_gain_eur: Some("-100") },
        EventExpectation { isin: "AT0000000002", symbol: "BBB", quantity: Some("2"), proceeds: Some("-200"), proceeds_eur: Some("-50"), total_quantity: "2", total_basis_eur: "50", net_gain_eur: None },
        EventExpectation { isin: "AT0000000001", symbol: "", quantity: None, proceeds: None, proceeds_eur: None, total_quantity: "9", total_basis_eur: "447", net_gain_eur: None },
        EventExpectation { isin: "AT0000000001", symbol: "AAA", quantity: Some("-9"), proceeds: Some("540"), proceeds_eur: Some("90"), total_quantity: "0", total_basis_eur: "0", net_gain_eur: Some("-357") },
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, expected) in rows.iter().zip(expected.iter()) {
        assert_event(row, &headers, expected);
    }

    let positions = fs::read_to_string(root.join("out/positions.csv")).expect("read positions");
    let mut reader = csv::Reader::from_reader(positions.as_bytes());
    let headers = reader.headers().unwrap().clone();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(value(&rows[0], &headers, "isin"), "AT0000000001");
    assert_eq!(value(&rows[0], &headers, "symbol"), "AAA");
    assert_eq!(decimal(&rows[0], &headers, "quantity"), Decimal::ZERO);
    assert_eq!(decimal(&rows[0], &headers, "basis_eur"), Decimal::ZERO);
    assert_eq!(value(&rows[1], &headers, "isin"), "AT0000000002");
    assert_eq!(value(&rows[1], &headers, "symbol"), "BBB");
    assert_eq!(decimal(&rows[1], &headers, "quantity"), Decimal::from_str("2").unwrap());
    assert_eq!(decimal(&rows[1], &headers, "basis_eur"), Decimal::from_str("50").unwrap());

    fs::remove_dir_all(root).expect("remove temporary directory");
}

#[test]
fn rejects_correction_quantity_difference_over_tolerance() {
    let root = temp_directory("failure");
    write_inputs(&root, "corrections-invalid.csv");
    let output = run(&root);
    assert!(!output.status.success(), "CLI unexpectedly succeeded: {output:?}");
    fs::remove_dir_all(root).expect("remove temporary directory");
}
