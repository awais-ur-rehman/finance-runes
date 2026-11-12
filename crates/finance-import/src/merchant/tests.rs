use super::{TransferHint, merchant, transfer_hint};

#[test]
fn a_card_purchase_shortens_to_the_merchant() {
    assert_eq!(
        merchant("POS SALE - VISA AT SAMPLE CAFE KARACHI PK"),
        Some("SAMPLE CAFE".to_owned()),
        "the city and country are dropped so a branch change keeps the rule"
    );
    assert_eq!(
        merchant("POS SALE - VISA AT SAMPLE MART LAHORE PK"),
        Some("SAMPLE MART".to_owned())
    );
}

#[test]
fn a_single_word_merchant_keeps_its_name() {
    assert_eq!(
        merchant("POS SALE - VISA AT NETFLIX KARACHI PK"),
        Some("NETFLIX".to_owned())
    );
}

#[test]
fn without_a_country_code_nothing_is_dropped() {
    assert_eq!(
        merchant("POS SALE - VISA AT SAMPLE CAFE"),
        Some("SAMPLE CAFE".to_owned()),
        "there is no signal that the last word is a city"
    );
}

#[test]
fn a_description_without_an_at_marker_has_no_merchant() {
    assert_eq!(merchant("ATM CASH WITHDRAWAL SAMPLE BRANCH"), None);
    assert_eq!(merchant("POS SALE - VISA AT "), None);
}

#[test]
fn a_cash_withdrawal_looks_like_a_transfer() {
    assert_eq!(
        transfer_hint("ATM CASH WITHDRAWAL SAMPLE BRANCH"),
        Some(TransferHint::CashWithdrawal),
        "the money is still the owner's, now in cash"
    );
}

#[test]
fn a_peer_transfer_names_its_counterparty() {
    assert_eq!(
        transfer_hint(
            "RA AST P2P FT FROM SAMPLE HOLDER HBL ACCT: PK00TEST0000. **103 MSGID: MSG000000000000000001"
        ),
        Some(TransferHint::PeerTransfer {
            counterparty: Some("SAMPLE HOLDER".to_owned()),
        })
    );
}

#[test]
fn a_peer_transfer_without_a_counterparty_is_still_a_hint() {
    assert_eq!(
        transfer_hint("P2P FT TO SAMPLE ACCOUNT"),
        Some(TransferHint::PeerTransfer { counterparty: None })
    );
}

#[test]
fn ordinary_spending_is_not_a_transfer() {
    assert_eq!(
        transfer_hint("POS SALE - VISA AT SAMPLE CAFE KARACHI PK"),
        None
    );
}
