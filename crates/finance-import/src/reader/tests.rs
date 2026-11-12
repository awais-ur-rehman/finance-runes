use super::{cell, is_opening_marker};

#[test]
fn the_opening_marker_ignores_case_and_spacing() {
    assert!(is_opening_marker("** Opening Balance"));
    assert!(is_opening_marker("  OPENING   BALANCE "));
}

#[test]
fn an_ordinary_description_is_not_an_opening_marker() {
    assert!(!is_opening_marker(
        "POS SALE - VISA AT OPENING CAFE KARACHI PK"
    ));
}

#[test]
fn a_cell_past_the_end_reads_as_empty() {
    let record = csv::StringRecord::from(vec!["only"]);

    assert_eq!(cell(&record, 0), "only");
    assert_eq!(
        cell(&record, 3),
        "",
        "the width is checked before this, and an empty cell beats a panic"
    );
}
