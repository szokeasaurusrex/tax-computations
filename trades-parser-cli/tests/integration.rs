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
    fs::write(root.join("opening.csv"), "isin,symbol,quantity,basis_eur\n")
        .expect("write empty opening positions");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let trades = root.join("trades");
    fs::create_dir_all(&trades).expect("create trade directory");
    fs::copy(
        fixtures.join("fake-trades.csv"),
        trades.join("fake-trades.csv"),
    )
    .expect("copy fake trades");
    fs::copy(fixtures.join("ecb.csv"), root.join("ecb.csv")).expect("copy fake ECB rates");
    fs::copy(fixtures.join(corrections), root.join("corrections.csv"))
        .expect("copy fake corrections");
    let corporate_actions = root.join("corporate-actions");
    fs::create_dir_all(&corporate_actions).expect("create corporate-action directory");
    fs::copy(
        fixtures.join("corporate-actions.csv"),
        corporate_actions.join("corporate-actions.csv"),
    )
    .expect("copy fake corporate actions");
}

fn run(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_trades-parser-cli"))
        .args([
            root.join("opening.csv"),
            root.join("trades"),
            root.join("ecb.csv"),
            root.join("corrections.csv"),
            root.join("corporate-actions"),
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
        expected
            .quantity
            .map(|value| Decimal::from_str(value).unwrap())
    );
    assert_eq!(
        optional_decimal(record, headers, "proceeds"),
        expected
            .proceeds
            .map(|value| Decimal::from_str(value).unwrap())
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
    assert_eq!(headers.get(0), Some("Transaction type"));
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(value(&rows[0], &headers, "Transaction type"), "Trade");
    assert_eq!(
        value(&rows[4], &headers, "Transaction type"),
        "Meldefonds Correction"
    );
    assert_eq!(
        value(&rows[5], &headers, "Transaction type"),
        "Corporate Action"
    );
    let expected = [
        EventExpectation {
            isin: "AT0000000001",
            symbol: "AAA",
            quantity: Some("10"),
            proceeds: Some("-1000"),
            proceeds_eur: Some("-500"),
            total_quantity: "10",
            total_basis_eur: "500",
            net_gain_eur: None,
        },
        EventExpectation {
            isin: "AT0000000001",
            symbol: "AAA",
            quantity: Some("5"),
            proceeds: Some("-600"),
            proceeds_eur: Some("-200"),
            total_quantity: "15",
            total_basis_eur: "700",
            net_gain_eur: None,
        },
        EventExpectation {
            isin: "AT0000000001",
            symbol: "AAA",
            quantity: Some("-6"),
            proceeds: Some("720"),
            proceeds_eur: Some("180"),
            total_quantity: "9",
            total_basis_eur: "420",
            net_gain_eur: Some("-100"),
        },
        EventExpectation {
            isin: "AT0000000002",
            symbol: "BBB",
            quantity: Some("2"),
            proceeds: Some("-200"),
            proceeds_eur: Some("-50"),
            total_quantity: "2",
            total_basis_eur: "50",
            net_gain_eur: None,
        },
        EventExpectation {
            isin: "AT0000000001",
            symbol: "",
            quantity: None,
            proceeds: None,
            proceeds_eur: None,
            total_quantity: "9",
            total_basis_eur: "447",
            net_gain_eur: None,
        },
        EventExpectation {
            isin: "AT0000000002",
            symbol: "",
            quantity: Some("3"),
            proceeds: None,
            proceeds_eur: None,
            total_quantity: "5",
            total_basis_eur: "50",
            net_gain_eur: None,
        },
        EventExpectation {
            isin: "AT0000000001",
            symbol: "AAA",
            quantity: Some("-9"),
            proceeds: Some("540"),
            proceeds_eur: Some("90"),
            total_quantity: "0",
            total_basis_eur: "0",
            net_gain_eur: Some("-357"),
        },
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
    let position_a = rows
        .iter()
        .find(|row| value(row, &headers, "isin") == "AT0000000001")
        .unwrap();
    assert_eq!(value(position_a, &headers, "symbol"), "AAA");
    assert_eq!(decimal(position_a, &headers, "quantity"), Decimal::ZERO);
    assert_eq!(decimal(position_a, &headers, "basis_eur"), Decimal::ZERO);
    let position_b = rows
        .iter()
        .find(|row| value(row, &headers, "isin") == "AT0000000002")
        .unwrap();
    assert_eq!(value(position_b, &headers, "symbol"), "BBB");
    assert_eq!(
        decimal(position_b, &headers, "quantity"),
        Decimal::from_str("5").unwrap()
    );
    assert_eq!(
        decimal(position_b, &headers, "basis_eur"),
        Decimal::from_str("50").unwrap()
    );

    fs::remove_dir_all(root).expect("remove temporary directory");
}

#[test]
fn continues_from_generated_positions() {
    let previous = temp_directory("previous-year");
    write_inputs(&previous, "corrections-valid.csv");
    assert!(run(&previous).status.success());

    let root = temp_directory("next-year");
    write_inputs(&root, "corrections-valid.csv");
    fs::copy(previous.join("out/positions.csv"), root.join("opening.csv")).unwrap();
    fs::write(
        root.join("trades/fake-trades.csv"),
        "CurrencyPrimary,Symbol,ISIN,DateTime,Quantity,Proceeds,Buy/Sell\n",
    )
    .unwrap();
    fs::write(
        root.join("corporate-actions/corporate-actions.csv"),
        "ISIN,Date/Time,Quantity\n",
    )
    .unwrap();
    fs::write(
        root.join("corrections.csv"),
        "ISIN,Report date,Shares on date (taxable),Correction/share (EUR)\n",
    )
    .unwrap();
    fs::write(
        root.join("ecb.csv"),
        "DATE,US dollar/Euro ECB reference exchange rate (EXR.D.USD.EUR.SP00.A)\n2027-07-20,2\n",
    )
    .unwrap();
    let output = run(&root);
    assert!(output.status.success(), "CLI failed: {output:?}");
    let read_positions = |path| {
        let mut positions = opening_positions_parser::read_csv(path)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        positions.sort_by(|a, b| a.isin.cmp(&b.isin));
        positions
    };
    let previous_positions = read_positions(previous.join("out/positions.csv"));
    assert_eq!(
        read_positions(root.join("out/positions.csv")),
        previous_positions,
    );

    fs::write(
        root.join("corrections.csv"),
        "ISIN,Report date,Shares on date (taxable),Correction/share (EUR)\nAT0000000002,2027-07-19,5,2\n",
    )
    .unwrap();
    fs::write(
        root.join("trades/fake-trades.csv"),
        "CurrencyPrimary,Symbol,ISIN,DateTime,Quantity,Proceeds,Buy/Sell\nUSD,BBB,AT0000000002,2027-07-20 09:00:00 UTC,-2,80,SELL\n",
    )
    .unwrap();
    let output = run(&root);
    assert!(output.status.success(), "CLI failed: {output:?}");
    let mut reader = csv::Reader::from_path(root.join("out/events.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.len(), 4);
    for row in &rows[..2] {
        assert_eq!(value(row, &headers, "Transaction type"), "Opening Position");
        let position = previous_positions
            .iter()
            .find(|position| position.isin == value(row, &headers, "ISIN"))
            .unwrap();
        assert_eq!(value(row, &headers, "symbol"), position.symbol);
        assert_eq!(decimal(row, &headers, "quantity"), position.quantity);
        assert_eq!(decimal(row, &headers, "total_quantity"), position.quantity);
        assert_eq!(
            decimal(row, &headers, "total_basis_eur"),
            position.basis_eur
        );
        for column in [
            "CurrencyPrimary",
            "DateTime",
            "proceeds",
            "Buy/Sell",
            "Proceeds (EUR)",
            "Report date",
            "Corporate action date",
            "Shares on date (taxable)",
            "Correction/share (EUR)",
            "net_gain_eur",
        ] {
            assert_eq!(value(row, &headers, column), "");
        }
    }
    assert_eq!(
        value(&rows[2], &headers, "Transaction type"),
        "Meldefonds Correction"
    );
    assert_eq!(value(&rows[3], &headers, "Transaction type"), "Trade");
    assert_eq!(
        decimal(&rows[2], &headers, "total_basis_eur"),
        Decimal::from(60)
    );
    assert_eq!(
        decimal(&rows[3], &headers, "total_quantity"),
        Decimal::from(3)
    );
    assert_eq!(
        decimal(&rows[3], &headers, "total_basis_eur"),
        Decimal::from(36)
    );
    assert_eq!(
        decimal(&rows[3], &headers, "net_gain_eur"),
        Decimal::from(16)
    );
    let positions = read_positions(root.join("out/positions.csv"));
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].quantity, Decimal::ZERO);
    assert_eq!(positions[1].quantity, Decimal::from(3));
    assert_eq!(positions[1].basis_eur, Decimal::from(36));

    fs::remove_dir_all(previous).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_opening_positions() {
    let root = temp_directory("invalid-opening");
    write_inputs(&root, "corrections-valid.csv");
    fs::write(
        root.join("opening.csv"),
        "isin,symbol,quantity,basis_eur\nAT0000000002,BBB,0,50\n",
    )
    .unwrap();
    let output = run(&root);
    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("InvalidPosition"),
        "unexpected error: {output:?}",
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_correction_quantity_difference_over_tolerance() {
    let root = temp_directory("failure");
    write_inputs(&root, "corrections-invalid.csv");
    let output = run(&root);
    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded: {output:?}"
    );
    fs::remove_dir_all(root).expect("remove temporary directory");
}
