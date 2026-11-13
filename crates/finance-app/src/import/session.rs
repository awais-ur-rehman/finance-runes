//! One import, from parsed rows to a committed batch.

use std::collections::{HashMap, HashSet};

use finance_core::{
    AccountId, Currency, Ledger, Money, PostingTarget, Transaction, TransactionId, TransactionKind,
    TransactionSource,
};
use finance_import::{DuplicateKey, Statement, StatementRow, TransferHint};

use crate::error::SessionError;
use crate::import::decision::{Decision, DuplicateState, Entry, SkipReason};
use crate::import::plan::{ImportPlan, ImportReport};
use crate::import::rule::Rule;

/// One row of the review list: what the file said, what is known about it, and
/// what the user decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRow {
    row: StatementRow,
    duplicate: Option<DuplicateState>,
    suggestion: Option<Entry>,
    decision: Decision,
    by_rule: bool,
}

impl ReviewRow {
    /// The 1-based line in the file, so the screen can point at it.
    pub const fn line(&self) -> usize {
        self.row.line()
    }

    /// The date the bank posted the row.
    pub const fn date(&self) -> time::Date {
        self.row.date()
    }

    /// The description exactly as the bank wrote it.
    pub fn description(&self) -> &str {
        self.row.description()
    }

    /// Which way the money moved and how much.
    pub const fn flow(&self) -> finance_import::Flow {
        self.row.flow()
    }

    /// The shortened merchant name, when the description carries one.
    pub fn merchant(&self) -> Option<String> {
        self.row.merchant()
    }

    /// The guess that this row moves money between the owner's own accounts.
    pub fn transfer_hint(&self) -> Option<TransferHint> {
        self.row.transfer_hint()
    }

    /// What makes this row a repeat, if it is one.
    pub const fn duplicate(&self) -> Option<DuplicateState> {
        self.duplicate
    }

    /// What a remembered rule would do with this row, regardless of the
    /// decision.
    pub const fn suggestion(&self) -> Option<Entry> {
        self.suggestion
    }

    /// The decision, which starts as a skip for a duplicate and undecided
    /// otherwise.
    pub const fn decision(&self) -> Decision {
        self.decision
    }

    /// True when the current decision came from a remembered rule rather than
    /// from the user. Clearing the rules returns these rows to undecided.
    pub const fn is_decided_by_rule(&self) -> bool {
        self.by_rule
    }
}

/// A statement under review, with one decision per row.
///
/// The session owns the statement and the decisions. It borrows the ledger only
/// while it is created, to recognise rows that are already recorded, and again
/// when it commits. Between those two moments nothing outside can change what
/// the user sees, so the preview, the plan and the commit cannot disagree.
#[derive(Clone, Debug)]
pub struct ImportSession {
    account: AccountId,
    statement: Statement,
    rows: Vec<ReviewRow>,
    rules: Vec<Rule>,
    records_opening: bool,
    committed: bool,
}

impl ImportSession {
    /// Opens a review of `statement` against `account`.
    ///
    /// Rows that already exist in the ledger are skipped by default, as FR-035
    /// asks; the user can still post them. Every other row starts undecided,
    /// because filing a row under a category nobody chose is worse than asking.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::UnknownAccount`] when the account is not in the
    /// ledger, [`SessionError::ArchivedAccount`] when it is archived, and
    /// [`SessionError::CurrencyMismatch`] when the statement is in another
    /// currency than the ledger.
    pub fn new(
        account: AccountId,
        statement: Statement,
        ledger: &Ledger,
    ) -> Result<Self, SessionError> {
        let Some(destination) = ledger.account(account) else {
            return Err(SessionError::UnknownAccount(account));
        };
        if destination.is_archived() {
            return Err(SessionError::ArchivedAccount(account));
        }
        if statement.currency() != ledger.currency() {
            return Err(SessionError::CurrencyMismatch {
                ledger: ledger.currency(),
                statement: statement.currency(),
            });
        }

        let existing = ledger_keys(ledger, account);
        let repeated: HashMap<usize, usize> = statement
            .duplicates()
            .iter()
            .map(|duplicate| (duplicate.line(), duplicate.first_line()))
            .collect();

        let rows = statement
            .rows()
            .iter()
            .map(|row| {
                let duplicate = if existing.contains(&comparison_key(row)) {
                    Some(DuplicateState::InLedger)
                } else {
                    repeated
                        .get(&row.line())
                        .map(|first_line| DuplicateState::InFile {
                            first_line: *first_line,
                        })
                };
                let decision = match duplicate {
                    Some(state) => Decision::Skip(SkipReason::Duplicate(state)),
                    None => Decision::Undecided,
                };
                ReviewRow {
                    row: row.clone(),
                    duplicate,
                    suggestion: None,
                    decision,
                    by_rule: false,
                }
            })
            .collect();

        let records_opening = statement.opening_balance().is_some()
            && !ledger
                .transactions()
                .any(|transaction| is_opening_for(transaction, account));

        Ok(Self {
            account,
            statement,
            rows,
            rules: Vec::new(),
            records_opening,
            committed: false,
        })
    }

    /// The account the rows will be filed under (FR-038).
    pub const fn account(&self) -> AccountId {
        self.account
    }

    /// The parsed statement, including its preview, issues and breaks.
    pub const fn statement(&self) -> &Statement {
        &self.statement
    }

    /// The review list, in file order.
    pub fn rows(&self) -> &[ReviewRow] {
        &self.rows
    }

    /// The remembered rules, oldest first.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Decides one row.
    ///
    /// A posted row teaches the session: its merchant or its transfer pattern is
    /// remembered and applied to later rows that look the same (FR-033). Rows
    /// already decided by the user are never overwritten by a rule.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::UnknownRow`] for an index outside the list, and
    /// [`SessionError::SameAccount`] when a transfer names the account being
    /// imported as its other side.
    pub fn set_decision(&mut self, index: usize, decision: Decision) -> Result<(), SessionError> {
        let row = self
            .rows
            .get(index)
            .ok_or(SessionError::UnknownRow { index })?;

        if decision == Decision::Post(Entry::Transfer(self.account)) {
            return Err(SessionError::SameAccount(self.account));
        }

        let learned = match decision {
            Decision::Post(entry) => Rule::learned_from(&row.row, entry),
            Decision::Skip(_) | Decision::Undecided => None,
        };

        let row = &mut self.rows[index];
        row.decision = decision;
        row.by_rule = false;

        if let Some(rule) = learned {
            self.remember(rule);
        }

        Ok(())
    }

    /// Adds a rule and applies it to the rows that have no decision yet.
    ///
    /// A rule about the same merchant or transfer pattern replaces the older
    /// one, so re-teaching a row changes the answer instead of stacking a second
    /// opinion behind the first.
    pub fn remember(&mut self, rule: Rule) {
        self.rules.retain(|existing| !existing.same_subject(&rule));
        self.rules.push(rule);
        self.apply_rules();
    }

    /// Forgets every rule and returns the rows those rules decided to undecided.
    ///
    /// The user's own choices and the duplicate skips are kept.
    pub fn clear_rules(&mut self) {
        self.rules.clear();
        self.apply_rules();
    }

    /// What a commit would write, given the decisions so far.
    ///
    /// Totals cover only the rows that will be posted, so the screen can show
    /// "112 new, in 45,000.00, out 32,500.00" the way FR-033 describes.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Overflow`] when a total does not fit in a 64-bit
    /// count of minor units. It cannot fire for a subset of a statement the
    /// importer already totalled, but wrapping a total is never the better
    /// answer.
    pub fn plan(&self) -> Result<ImportPlan, SessionError> {
        let currency = self.statement.currency();
        let mut duplicates = 0;
        let mut to_post = 0;
        let mut skipped = 0;
        let mut undecided = 0;
        let mut transfers = 0;
        let mut incoming: i128 = 0;
        let mut outgoing: i128 = 0;

        for row in &self.rows {
            if row.duplicate.is_some() {
                duplicates += 1;
            }
            match row.decision {
                Decision::Undecided => undecided += 1,
                Decision::Skip(_) => skipped += 1,
                Decision::Post(entry) => {
                    to_post += 1;
                    if matches!(entry, Entry::Transfer(_)) {
                        transfers += 1;
                    }
                    let amount = i128::from(row.row.flow().amount().minor());
                    if row.row.flow().is_incoming() {
                        incoming += amount;
                    } else {
                        outgoing += amount;
                    }
                }
            }
        }

        Ok(ImportPlan {
            rows: self.rows.len(),
            duplicates,
            to_post,
            skipped,
            undecided,
            transfers,
            total_in: money(incoming, currency)?,
            total_out: money(outgoing, currency)?,
            opening: self.statement.opening_balance().cloned(),
            records_opening: self.records_opening,
        })
    }

    /// Writes every posted row and the opening balance in one batch.
    ///
    /// The batch is all or nothing (FR-034): if the ledger refuses any row, none
    /// of them is written and the session stays usable, so the user can fix the
    /// row and try again.
    ///
    /// `next_id` supplies a transaction id for each row; the session never
    /// generates one itself, so a test can predict every id it writes.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Undecided`] while any row has no decision,
    /// [`SessionError::AlreadyCommitted`] on a second commit,
    /// [`SessionError::Transaction`] when a row cannot be built, and
    /// [`SessionError::Ledger`] when the ledger refuses the batch. In every case
    /// nothing was written.
    pub fn commit(
        &mut self,
        ledger: &mut Ledger,
        mut next_id: impl FnMut() -> TransactionId,
    ) -> Result<ImportReport, SessionError> {
        if self.committed {
            return Err(SessionError::AlreadyCommitted);
        }

        let undecided = self
            .rows
            .iter()
            .filter(|row| matches!(row.decision, Decision::Undecided))
            .count();
        if undecided > 0 {
            return Err(SessionError::Undecided { rows: undecided });
        }

        let currency = self.statement.currency();
        let mut transactions = Vec::with_capacity(self.rows.len() + 1);
        let mut posted = 0;
        let mut skipped = 0;
        let mut transfers = 0;
        let mut incoming: i128 = 0;
        let mut outgoing: i128 = 0;

        let opening = match self.statement.opening_balance() {
            Some(opening) if self.records_opening => {
                transactions.push(
                    Transaction::opening(
                        next_id(),
                        opening.date(),
                        self.account,
                        opening.amount(),
                    )?
                    .with_source(self.source()),
                );
                Some(opening.amount())
            }
            _ => None,
        };

        for row in &self.rows {
            let Decision::Post(entry) = row.decision else {
                skipped += 1;
                continue;
            };

            transactions.push(build(
                next_id(),
                self.account,
                &row.row,
                entry,
                self.statement.profile(),
            )?);
            posted += 1;
            if matches!(entry, Entry::Transfer(_)) {
                transfers += 1;
            }
            let amount = i128::from(row.row.flow().amount().minor());
            if row.row.flow().is_incoming() {
                incoming += amount;
            } else {
                outgoing += amount;
            }
        }

        ledger.record_all(transactions)?;
        self.committed = true;

        Ok(ImportReport::new(
            posted,
            skipped,
            transfers,
            opening,
            money(incoming, currency)?,
            money(outgoing, currency)?,
        ))
    }

    /// The provenance every row this session writes carries (FR-036).
    fn source(&self) -> TransactionSource {
        TransactionSource::Imported {
            profile: self.statement.profile().to_owned(),
        }
    }

    /// Recomputes every suggestion and fills in rows the rules decide.
    ///
    /// A row keeps its decision when the user made it; a row that a rule decided
    /// follows the rules, including back to undecided when no rule matches any
    /// more.
    fn apply_rules(&mut self) {
        let rules = &self.rules;
        for row in &mut self.rows {
            row.suggestion = rules.iter().rev().find_map(|rule| rule.apply(&row.row));

            let follows_rules = row.by_rule || matches!(row.decision, Decision::Undecided);
            if !follows_rules {
                continue;
            }

            match row.suggestion {
                Some(entry) => {
                    row.decision = Decision::Post(entry);
                    row.by_rule = true;
                }
                None => {
                    row.decision = Decision::Undecided;
                    row.by_rule = false;
                }
            }
        }
    }
}

/// The comparison key for a row: day, signed amount and normalised description.
///
/// The instrument number is deliberately not part of it. The ledger keeps the
/// raw description as the note but not the bank's reference, so a cross-check
/// has to use the key both sides can build.
fn comparison_key(row: &StatementRow) -> DuplicateKey {
    DuplicateKey::comparison(row.date(), row.flow().signed(), row.description())
}

/// The comparison keys of everything already recorded against `account`.
fn ledger_keys(ledger: &Ledger, account: AccountId) -> HashSet<DuplicateKey> {
    ledger
        .transactions()
        .filter(|transaction| transaction.kind() != TransactionKind::Opening)
        .filter_map(|transaction| DuplicateKey::for_transaction(transaction, account))
        .collect()
}

/// True when the transaction is this account's opening balance.
fn is_opening_for(transaction: &Transaction, account: AccountId) -> bool {
    transaction.kind() == TransactionKind::Opening
        && transaction
            .postings()
            .iter()
            .any(|posting| posting.target() == PostingTarget::Account(account))
}

/// Builds the transaction a posted row becomes.
///
/// The note keeps the raw description so the register shows what the bank wrote
/// and so a re-import can recognise the row (FR-035, FR-036). The payee is the
/// merchant hint, which is what the user scans for.
fn build(
    id: TransactionId,
    account: AccountId,
    row: &StatementRow,
    entry: Entry,
    profile: &str,
) -> Result<Transaction, SessionError> {
    let amount = row.flow().amount();
    let transaction = match entry {
        Entry::Category(category) => {
            if row.flow().is_incoming() {
                Transaction::income(id, row.date(), account, category, amount)?
            } else {
                Transaction::expense(id, row.date(), account, category, amount)?
            }
        }
        Entry::Transfer(other) => {
            if row.flow().is_incoming() {
                Transaction::transfer(id, row.date(), other, account, amount)?
            } else {
                Transaction::transfer(id, row.date(), account, other, amount)?
            }
        }
    }
    .with_source(TransactionSource::Imported {
        profile: profile.to_owned(),
    });

    Ok(match row.merchant() {
        Some(merchant) => transaction
            .with_payee(merchant)
            .with_note(row.description()),
        None => transaction.with_note(row.description()),
    })
}

/// Converts a running total into money, refusing to wrap.
fn money(minor: i128, currency: Currency) -> Result<Money, SessionError> {
    i64::try_from(minor)
        .map(|minor| Money::from_minor(minor, currency))
        .map_err(|_| SessionError::Overflow)
}
