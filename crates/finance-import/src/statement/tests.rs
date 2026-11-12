use finance_core::{Currency, Money};

use super::Flow;

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

#[test]
fn a_flow_keeps_direction_out_of_the_amount() {
    let out = Flow::outgoing(pkr(84_000)).expect("the amount is positive");

    assert_eq!(out.amount(), pkr(84_000));
    assert!(out.is_outgoing());
    assert!(!out.is_incoming());
    assert_eq!(out.signed(), pkr(-84_000));
}

#[test]
fn an_incoming_flow_is_positive_when_signed() {
    let incoming = Flow::incoming(pkr(500_000)).expect("the amount is positive");

    assert_eq!(incoming.signed(), pkr(500_000));
}

#[test]
fn a_flow_refuses_a_non_positive_amount() {
    assert!(Flow::incoming(pkr(0)).is_none());
    assert!(Flow::outgoing(pkr(-1)).is_none());
}
