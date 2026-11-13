//! Behaviour of the import review, from the confirmed statement shape to a
//! committed batch.
//!
//! Every row is synthetic: invented names, amounts and reference numbers. The
//! real statement is not in this repository and never will be. See
//! `docs/requirements/csv-import-profile.md`.

use finance_core::{
    Account, AccountId, AccountKind, Category, CategoryId, CategoryKind, Currency, Ledger,
    LedgerError, Money, Transaction, TransactionId, TransactionKind, TransactionSource,
};
use finance_import::{Profile, parse};
use time::{Date, Month};
use uuid::Uuid;

use super::{Decision, DuplicateState, Entry, ImportSession, Rule, SkipReason};
use crate::SessionError;

const FIXTURE: &str = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1250.00
12-Aug-2026,RA AST P2P FT FROM SAMPLE HOLDER HBL ACCT: PK00TEST0000. **103 MSGID: MSG000000000000000001,,0,5000.00,6250.00
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,5410.00
13-Aug-2026,POS SALE - VISA AT SAMPLE MART LAHORE PK,,1200.50,0,4209.50
13-Aug-2026,ATM CASH WITHDRAWAL SAMPLE BRANCH,,2000.00,0,2209.50
";

/// Two card rows at the same merchant in different cities, which is what the
/// merchant rule has to survive.
const MERCHANTS: &str = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,10000.00
12-Aug-2026,POS SALE - VISA AT SAMPLE CAFE KARACHI PK,,840.00,0,9160.00
13-Aug-2026,POS SALE - VISA AT SAMPLE CAFE LAHORE PK,,500.00,0,8660.00
";

/// Two cash withdrawals, which share a transfer pattern but not a merchant.
const WITHDRAWALS: &str = "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,10000.00
12-Aug-2026,ATM CASH WITHDRAWAL SAMPLE BRANCH,,2000.00,0,8000.00
13-Aug-2026,ATM CASH WITHDRAWAL OTHER BRANCH,,1000.00,0,7000.00
";

struct Fixture {
    ledger: Ledger,
    bank: AccountId,
    primary: AccountId,
    cash: AccountId,
    groceries: CategoryId,
    salary: CategoryId,
}

fn fixture() -> Fixture {
    let mut ledger = Ledger::new(Currency::PKR);
    let bank = account_id(1);
    let primary = account_id(2);
    let cash = account_id(3);
    ledger
        .add_account(Account::new(
            bank,
            "Pass-through",
            AccountKind::Bank,
            Currency::PKR,
        ))
        .expect("the account is unique");
    ledger
        .add_account(Account::new(
            primary,
            "Primary",
            AccountKind::Bank,
            Currency::PKR,
        ))
        .expect("the account is unique");
    ledger
        .add_account(Account::new(cash, "Cash", AccountKind::Cash, Currency::PKR))
        .expect("the account is unique");

    let groceries = category_id(1);
    let salary = category_id(2);
    ledger
        .add_category(Category::new(groceries, "Groceries", CategoryKind::Expense))
        .expect("the category is unique");
    ledger
        .add_category(Category::new(salary, "Salary", CategoryKind::Income))
        .expect("the category is unique");

    Fixture {
        ledger,
        bank,
        primary,
        cash,
        groceries,
        salary,
    }
}

fn account_id(value: u128) -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(value))
}

fn category_id(value: u128) -> CategoryId {
    CategoryId::from_uuid(Uuid::from_u128(value))
}

fn transaction_id(value: u128) -> TransactionId {
    TransactionId::from_uuid(Uuid::from_u128(value))
}

fn pkr(minor: i64) -> Money {
    Money::from_minor(minor, Currency::PKR)
}

fn day(number: u8) -> Date {
    Date::from_calendar_date(2026, Month::August, number).expect("the test date is valid")
}

fn profile() -> Profile {
    Profile::bank_statement_debit_credit()
}

fn review(fixture: &Fixture, csv: &str) -> ImportSession {
    let statement = parse(csv.as_bytes(), &profile(), Currency::PKR).expect("the fixture parses");
    ImportSession::new(fixture.bank, statement, &fixture.ledger)
        .expect("the destination account is in the ledger")
}

fn session(fixture: &Fixture) -> ImportSession {
    review(fixture, FIXTURE)
}

/// Sequential identifiers, so a test can name every transaction it writes.
fn ids() -> impl FnMut() -> TransactionId {
    let mut next = 1_u128;
    move || {
        let id = transaction_id(next);
        next += 1;
        id
    }
}

fn decide_all(session: &mut ImportSession, fixture: &Fixture) -> Result<(), SessionError> {
    session.set_decision(0, Decision::Post(Entry::Transfer(fixture.primary)))?;
    session.set_decision(1, Decision::Post(Entry::Category(fixture.groceries)))?;
    session.set_decision(2, Decision::Post(Entry::Category(fixture.groceries)))?;
    session.set_decision(3, Decision::Post(Entry::Transfer(fixture.cash)))?;
    Ok(())
}

#[test]
fn a_new_review_starts_with_one_undecided_row_per_transaction() {
    let fixture = fixture();
    let session = session(&fixture);

    assert_eq!(session.rows().len(), 4, "the opening marker is not a row");
    assert!(
        session
            .rows()
            .iter()
            .all(|row| row.decision() == Decision::Undecided),
        "filing a row under a category nobody chose is worse than asking"
    );

    let plan = session.plan().expect("the totals fit");
    assert_eq!(plan.rows(), 4);
    assert_eq!(plan.undecided(), 4);
    assert_eq!(plan.duplicates(), 0);
    assert_eq!(plan.to_post(), 0);
    assert_eq!(plan.total_in(), pkr(0));
    assert_eq!(plan.total_out(), pkr(0));
    assert_eq!(
        plan.opening().expect("the marker row").amount(),
        pkr(125_000)
    );
    assert!(plan.records_opening());
    assert!(!plan.is_ready());
}

#[test]
fn a_row_already_in_the_ledger_is_skipped_by_default() {
    let mut fixture = fixture();
    fixture
        .ledger
        .record(
            Transaction::expense(
                transaction_id(1),
                day(12),
                fixture.bank,
                fixture.groceries,
                pkr(84_000),
            )
            .expect("the amount is positive")
            .with_note("POS SALE - VISA AT SAMPLE CAFE KARACHI PK"),
        )
        .expect("the expense is valid");

    let session = session(&fixture);
    let row = &session.rows()[1];

    assert_eq!(row.duplicate(), Some(DuplicateState::InLedger));
    assert_eq!(
        row.decision(),
        Decision::Skip(SkipReason::Duplicate(DuplicateState::InLedger))
    );

    let plan = session.plan().expect("the totals fit");
    assert_eq!(plan.duplicates(), 1);
    assert_eq!(plan.skipped(), 1);
    assert_eq!(plan.undecided(), 3);
}

#[test]
fn a_row_repeated_inside_the_file_is_skipped_by_default() {
    let fixture = fixture();
    let session = review(
        &fixture,
        "\
DATE,PARTICULARS,INST. NO.,DEBIT,CREDIT,BALANCE
12-Aug-2026,** Opening Balance,,0,0,1250.00
12-Aug-2026,ROW A,,0,1000.00,2250.00
12-Aug-2026,ROW A,,0,1000.00,3250.00
",
    );

    let repeated = DuplicateState::InFile { first_line: 3 };
    assert_eq!(session.rows()[1].duplicate(), Some(repeated));
    assert_eq!(
        session.rows()[1].decision(),
        Decision::Skip(SkipReason::Duplicate(repeated))
    );
    assert_eq!(session.rows()[0].duplicate(), None);
}

#[test]
fn a_merchant_choice_is_preselected_for_later_rows() {
    let fixture = fixture();
    let mut session = review(&fixture, MERCHANTS);

    session
        .set_decision(0, Decision::Post(Entry::Category(fixture.groceries)))
        .expect("the row exists");

    assert_eq!(session.rules().len(), 1, "the merchant was remembered");
    assert_eq!(
        session.rows()[1].suggestion(),
        Some(Entry::Category(fixture.groceries)),
        "the city changed but the merchant did not"
    );
    assert_eq!(
        session.rows()[1].decision(),
        Decision::Post(Entry::Category(fixture.groceries))
    );
    assert!(session.rows()[1].is_decided_by_rule());
    assert!(
        !session.rows()[0].is_decided_by_rule(),
        "the row the user decided is not a suggestion"
    );
}

#[test]
fn a_cash_withdrawal_choice_is_preselected_for_later_withdrawals() {
    let fixture = fixture();
    let mut session = review(&fixture, WITHDRAWALS);

    session
        .set_decision(0, Decision::Post(Entry::Transfer(fixture.cash)))
        .expect("the row exists");

    assert!(matches!(session.rules()[0], Rule::Transfer { .. }));
    assert_eq!(
        session.rows()[1].decision(),
        Decision::Post(Entry::Transfer(fixture.cash))
    );
}

#[test]
fn filing_a_transfer_row_as_a_category_teaches_no_rule() {
    let fixture = fixture();
    let mut session = review(&fixture, WITHDRAWALS);

    session
        .set_decision(0, Decision::Post(Entry::Category(fixture.groceries)))
        .expect("the row exists");

    assert!(
        session.rules().is_empty(),
        "a withdrawal can be spending, but it must not teach every withdrawal to be"
    );
    assert_eq!(session.rows()[1].decision(), Decision::Undecided);
}

#[test]
fn re_teaching_a_merchant_replaces_the_earlier_rule() {
    let fixture = fixture();
    let mut session = review(&fixture, MERCHANTS);

    session
        .set_decision(0, Decision::Post(Entry::Category(fixture.groceries)))
        .expect("the row exists");
    session
        .set_decision(0, Decision::Post(Entry::Category(fixture.salary)))
        .expect("the row exists");

    assert_eq!(session.rules().len(), 1, "one opinion per merchant");
    assert_eq!(
        session.rows()[1].decision(),
        Decision::Post(Entry::Category(fixture.salary))
    );
}

#[test]
fn clearing_the_rules_returns_suggested_rows_to_undecided() {
    let fixture = fixture();
    let mut session = review(&fixture, MERCHANTS);

    session
        .set_decision(0, Decision::Post(Entry::Category(fixture.groceries)))
        .expect("the row exists");
    session.clear_rules();

    assert!(session.rules().is_empty());
    assert_eq!(
        session.rows()[0].decision(),
        Decision::Post(Entry::Category(fixture.groceries)),
        "a decision the user made is not a suggestion"
    );
    assert_eq!(session.rows()[1].decision(), Decision::Undecided);
    assert_eq!(session.rows()[1].suggestion(), None);
}

#[test]
fn a_decision_for_a_row_that_does_not_exist_is_refused() {
    let fixture = fixture();
    let mut session = session(&fixture);

    let error = session
        .set_decision(99, Decision::Post(Entry::Category(fixture.groceries)))
        .expect_err("there is no row 99");

    assert!(matches!(error, SessionError::UnknownRow { index: 99 }));
}

#[test]
fn a_transfer_to_the_imported_account_is_refused() {
    let fixture = fixture();
    let mut session = session(&fixture);

    let error = session
        .set_decision(0, Decision::Post(Entry::Transfer(fixture.bank)))
        .expect_err("money cannot be moved to where it already is");

    assert!(matches!(error, SessionError::SameAccount(id) if id == fixture.bank));
}

#[test]
fn a_skipped_row_is_left_out_of_the_totals() {
    let fixture = fixture();
    let mut session = session(&fixture);
    decide_all(&mut session, &fixture).expect("the rows exist");
    session
        .set_decision(0, Decision::Skip(SkipReason::Chosen))
        .expect("the row exists");

    let plan = session.plan().expect("the totals fit");
    assert_eq!(plan.skipped(), 1);
    assert_eq!(plan.to_post(), 3);
    assert_eq!(plan.total_in(), pkr(0), "the transfer is no longer posted");
    assert_eq!(plan.total_out(), pkr(404_050));
    assert!(plan.is_ready());
}

#[test]
fn committing_writes_the_rows_and_the_opening_balance() {
    let mut fixture = fixture();
    let mut session = session(&fixture);
    decide_all(&mut session, &fixture).expect("the rows exist");

    let report = session
        .commit(&mut fixture.ledger, ids())
        .expect("every row is decided and valid");

    assert_eq!(report.posted(), 4);
    assert_eq!(report.skipped(), 0);
    assert_eq!(report.transfers(), 2);
    assert_eq!(report.opening(), Some(pkr(125_000)));
    assert_eq!(report.total_in(), pkr(500_000));
    assert_eq!(report.total_out(), pkr(404_050));

    assert_eq!(fixture.ledger.transactions().count(), 5);

    let opening = fixture
        .ledger
        .transactions()
        .find(|transaction| transaction.kind() == TransactionKind::Opening)
        .expect("the opening balance was written");
    assert_eq!(
        opening.source(),
        &TransactionSource::Imported {
            profile: "bank-statement-debit-credit".to_owned()
        },
        "the opening balance came from the file too"
    );

    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(13))
            .expect("the account exists"),
        pkr(220_950),
        "the file's final balance"
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.cash, day(13))
            .expect("the account exists"),
        pkr(200_000),
        "the withdrawal is still the owner's money"
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.primary, day(13))
            .expect("the account exists"),
        pkr(-500_000)
    );
}

#[test]
fn an_imported_row_keeps_the_bank_description_and_the_merchant() {
    let mut fixture = fixture();
    let mut session = session(&fixture);
    decide_all(&mut session, &fixture).expect("the rows exist");
    session
        .commit(&mut fixture.ledger, ids())
        .expect("every row is decided and valid");

    let cafe = fixture
        .ledger
        .transactions()
        .find(|transaction| transaction.payee() == Some("SAMPLE CAFE"))
        .expect("the card row was imported");

    assert_eq!(
        cafe.note(),
        Some("POS SALE - VISA AT SAMPLE CAFE KARACHI PK"),
        "the register shows what the bank wrote"
    );
    assert_eq!(cafe.date(), day(12));
    assert_eq!(cafe.amount(), pkr(84_000));
    assert_eq!(
        cafe.source(),
        &TransactionSource::Imported {
            profile: "bank-statement-debit-credit".to_owned()
        },
        "an imported row says where it came from (FR-036)"
    );
}

#[test]
fn committing_refuses_while_a_row_is_undecided() {
    let mut fixture = fixture();
    let mut session = session(&fixture);
    session
        .set_decision(0, Decision::Post(Entry::Transfer(fixture.primary)))
        .expect("the row exists");

    let error = session
        .commit(&mut fixture.ledger, ids())
        .expect_err("three rows still need a decision");

    assert!(matches!(error, SessionError::Undecided { rows: 3 }));
    assert_eq!(
        fixture.ledger.transactions().count(),
        0,
        "a refused commit writes nothing"
    );
}

#[test]
fn committing_twice_is_refused() {
    let mut fixture = fixture();
    let mut session = session(&fixture);
    decide_all(&mut session, &fixture).expect("the rows exist");

    session
        .commit(&mut fixture.ledger, ids())
        .expect("the first commit succeeds");
    let error = session
        .commit(&mut fixture.ledger, ids())
        .expect_err("the rows are already written");

    assert!(matches!(error, SessionError::AlreadyCommitted));
    assert_eq!(fixture.ledger.transactions().count(), 5);
}

#[test]
fn a_commit_the_ledger_refuses_writes_nothing() {
    let mut fixture = fixture();
    let mut session = session(&fixture);
    decide_all(&mut session, &fixture).expect("the rows exist");
    session
        .set_decision(1, Decision::Post(Entry::Category(fixture.salary)))
        .expect("the row exists");

    let error = session
        .commit(&mut fixture.ledger, ids())
        .expect_err("an expense cannot use an income category");

    assert!(matches!(
        error,
        SessionError::Ledger(LedgerError::CategoryKindMismatch {
            expected: CategoryKind::Expense,
            found: CategoryKind::Income
        })
    ));
    assert_eq!(
        fixture.ledger.transactions().count(),
        0,
        "the first rows must not survive a failed batch"
    );

    session
        .set_decision(1, Decision::Post(Entry::Category(fixture.groceries)))
        .expect("the row exists");
    session
        .commit(&mut fixture.ledger, ids())
        .expect("the corrected batch succeeds");
    assert_eq!(fixture.ledger.transactions().count(), 5);
}

#[test]
fn an_account_that_already_has_an_opening_balance_gets_no_second() {
    let mut fixture = fixture();
    fixture
        .ledger
        .record(
            Transaction::opening(transaction_id(100), day(12), fixture.bank, pkr(125_000))
                .expect("the amount is positive"),
        )
        .expect("the opening balance is valid");

    let mut session = session(&fixture);
    let plan = session.plan().expect("the totals fit");
    assert!(!plan.records_opening());
    assert!(plan.opening().is_some(), "the file still carries one");

    decide_all(&mut session, &fixture).expect("the rows exist");
    let report = session
        .commit(&mut fixture.ledger, ids())
        .expect("every row is decided and valid");

    assert_eq!(report.opening(), None);
    assert_eq!(
        fixture.ledger.transactions().count(),
        5,
        "one opening balance and four rows"
    );
    assert_eq!(
        fixture
            .ledger
            .balance_as_of(fixture.bank, day(13))
            .expect("the account exists"),
        pkr(220_950),
        "the starting money was not counted twice"
    );
}

#[test]
fn an_unknown_destination_account_is_refused() {
    let fixture = fixture();
    let statement =
        parse(FIXTURE.as_bytes(), &profile(), Currency::PKR).expect("the fixture parses");

    let error = ImportSession::new(account_id(99), statement, &fixture.ledger)
        .expect_err("the account is not in the ledger");

    assert!(matches!(error, SessionError::UnknownAccount(id) if id == account_id(99)));
}

#[test]
fn an_archived_destination_account_is_refused() {
    let mut ledger = Ledger::new(Currency::PKR);
    let bank = account_id(1);
    let mut account = Account::new(bank, "Old bank", AccountKind::Bank, Currency::PKR);
    account.archive();
    ledger.add_account(account).expect("the account is unique");
    let statement =
        parse(FIXTURE.as_bytes(), &profile(), Currency::PKR).expect("the fixture parses");

    let error = ImportSession::new(bank, statement, &ledger)
        .expect_err("an archived account takes no new rows");

    assert!(matches!(error, SessionError::ArchivedAccount(id) if id == bank));
}

#[test]
fn a_statement_in_another_currency_is_refused() {
    let fixture = fixture();
    let statement =
        parse(FIXTURE.as_bytes(), &profile(), Currency::USD).expect("the fixture parses");

    let error = ImportSession::new(fixture.bank, statement, &fixture.ledger)
        .expect_err("the currencies differ");

    assert!(matches!(
        error,
        SessionError::CurrencyMismatch {
            ledger: Currency::PKR,
            statement: Currency::USD
        }
    ));
}
