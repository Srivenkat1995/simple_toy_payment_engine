use std::collections::HashMap;
use csv::Writer;
use log::{debug, warn, info};

use crate::accounts::client_account::ClientAccount;
use crate::transactions::{TransactionRecord, Transaction, TransactionType};

pub struct PaymentEngine {
    pub accounts: HashMap<u16, ClientAccount>,
    pub transactions: HashMap<u32, Transaction>,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Get mutable reference to a client account, or create new one if it doesn't exist
    fn get_account(&mut self, client_id: u16) -> &mut ClientAccount {
        self.accounts
            .entry(client_id)
            .or_insert_with(|| ClientAccount::new(client_id))
    }

    /// Process a single transaction
    pub fn process_transaction(&mut self, record: TransactionRecord) {
        match record.tx_type {
            TransactionType::Deposit => {
                if let Some(amount) = record.amount {
                    let account = self.get_account(record.client);
                    account.deposit(amount);

                    if self.transactions.contains_key(&record.tx) {
                        // Duplicate tx id: we've chosen to apply the new operation but log the overwrite
                        warn!("duplicate tx id {} for client {}: overwriting existing transaction", record.tx, record.client);
                    } else {
                        info!("recording deposit tx {} for client {} amount {}", record.tx, record.client, amount);
                    }

                    self.transactions.insert(
                        record.tx,
                        Transaction {
                            client: record.client,
                            amount,
                            disputed: false,
                        },
                    );
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = record.amount {
                    let account = self.get_account(record.client);
                    // Only record the transaction if the withdrawal actually succeeded
                    if account.withdraw(amount) {
                        self.transactions.insert(
                            record.tx,
                            Transaction {
                                client: record.client,
                                amount,
                                disputed: false,
                            },
                        );
                    } else {
                        // Log that a withdrawal failed and therefore was not recorded
                        debug!("withdrawal failed or account locked for client {} tx {} amount {}", record.client, record.tx, amount);
                    }
                }
            }
            TransactionType::Dispute
            | TransactionType::Resolve
            | TransactionType::Chargeback => {
                // Take the transaction out to avoid multiple mutable borrows
                if let Some(mut tx) = self.transactions.remove(&record.tx) {
                    let client_id = tx.client;
                    let amount = tx.amount;

                    // Borrow the account separately
                    let account = self.get_account(client_id);

                    match record.tx_type {
                        TransactionType::Dispute => {
                            if !tx.disputed {
                                account.hold(amount);
                                tx.disputed = true;
                            }
                        }
                        TransactionType::Resolve => {
                            if tx.disputed {
                                account.release(amount);
                                tx.disputed = false;
                            } else {
                                // Resolve attempted on a transaction that isn't disputed
                                warn!("resolve attempted for tx {} which is not under dispute", record.tx);
                            }
                        }
                        TransactionType::Chargeback => {
                            if tx.disputed {
                                account.chargeback(amount);
                                tx.disputed = false;
                            } else {
                                // Chargeback attempted on a transaction that isn't disputed
                                warn!("chargeback attempted for tx {} which is not under dispute", record.tx);
                            }
                        }
                        _ => {}
                    }

                    // Put the transaction back into the map
                    self.transactions.insert(record.tx, tx);
                } else {
                    // Transaction not found — log and ignore as per spec
                    debug!("ignoring {:?} for tx {}: transaction not found", record.tx_type, record.tx);
                }
            }
        }
    }

    /// Output all accounts to stdout in CSV format
    pub fn output_accounts(&self) {
        let mut wtr = Writer::from_writer(std::io::stdout());
        wtr.write_record(&["client", "available", "held", "total", "locked"])
            .unwrap();

        for account in self.accounts.values() {
            wtr.serialize((
                account.client,
                account.available.round_dp(4),
                account.held.round_dp(4),
                account.total.round_dp(4),
                account.locked,
            ))
            .unwrap();
        }

        wtr.flush().unwrap();
    }
}


/// ------------------------
/// Inline Unit Tests
/// ------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    fn decimal(amount: f64) -> Decimal {
        Decimal::from_f64(amount).unwrap()
    }

    fn deposit(client: u16, tx: u32, amount: f64) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Deposit,
            client,
            tx,
            amount: Some(decimal(amount)),
        }
    }

    fn withdrawal(client: u16, tx: u32, amount: f64) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Withdrawal,
            client,
            tx,
            amount: Some(decimal(amount)),
        }
    }

    fn dispute(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Dispute,
            client,
            tx,
            amount: None,
        }
    }

    fn resolve(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Resolve,
            client,
            tx,
            amount: None,
        }
    }

    fn chargeback(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Chargeback,
            client,
            tx,
            amount: None,
        }
    }

    #[test]
    fn test_deposit_and_withdrawal() {
        let mut engine = PaymentEngine::new();
        engine.process_transaction(deposit(1, 1, 100.0));
        engine.process_transaction(withdrawal(1, 2, 50.0));
        let acc = engine.get_account(1);
        assert_eq!(acc.available, decimal(50.0));
        assert_eq!(acc.total, decimal(50.0));
        assert!(!acc.locked);
    }

    #[test]
    fn test_insufficient_withdrawal() {
        let mut engine = PaymentEngine::new();
        engine.process_transaction(deposit(2, 1, 100.0));
        engine.process_transaction(withdrawal(2, 2, 200.0)); // should fail
        let acc = engine.get_account(2);
        assert_eq!(acc.available, decimal(100.0));
        assert_eq!(acc.total, decimal(100.0));
    }


    #[test]
    fn test_chargeback_locks_account() {
        let mut engine = PaymentEngine::new();
        engine.process_transaction(deposit(2, 1, 200.0));
        engine.process_transaction(dispute(2, 1));
        engine.process_transaction(chargeback(2, 1));

        let acc = engine.get_account(2);
        assert_eq!(acc.available, decimal(0.0));
        assert_eq!(acc.held, decimal(0.0));
        assert_eq!(acc.total, decimal(0.0));
        assert!(acc.locked);
    }

    #[test]
    fn test_decimal_precision() {
        let mut engine = PaymentEngine::new();
        engine.process_transaction(deposit(3, 1, 500.1234));
        engine.process_transaction(withdrawal(3, 2, 123.1234));

        let acc = engine.get_account(3);
        assert_eq!(acc.available.round_dp(4), decimal(377.0000));
        assert_eq!(acc.total.round_dp(4), decimal(377.0000));
    }

    #[test]
    fn test_multiple_clients() {
        let mut engine = PaymentEngine::new();
        engine.process_transaction(deposit(1, 1, 100.0));
        engine.process_transaction(deposit(2, 2, 200.0));
        engine.process_transaction(withdrawal(1, 3, 50.0));
        engine.process_transaction(withdrawal(2, 4, 50.0));

        // Immutable access for checks
        let acc1 = engine.accounts.get(&1).unwrap();
        let acc2 = engine.accounts.get(&2).unwrap();

        assert_eq!(acc1.available, decimal(50.0));
        assert_eq!(acc2.available, decimal(150.0));
    }

    #[test]
    fn test_invalid_dispute_resolve_chargeback() {
        let mut engine = PaymentEngine::new();
        // These should be ignored (tx does not exist)
        engine.process_transaction(dispute(1, 999));
        engine.process_transaction(resolve(1, 999));
        engine.process_transaction(chargeback(1, 999));

        // Account should not exist yet
        assert!(engine.accounts.get(&1).is_none());
    }

    #[test]
    fn test_dispute_on_failed_withdrawal_ignored() {
        let mut engine = PaymentEngine::new();

        // Deposit 50
        engine.process_transaction(deposit(1, 1, 50.0));

        // Withdrawal of 100 should fail and not be recorded
        engine.process_transaction(withdrawal(1, 2, 100.0));
        assert!(engine.transactions.get(&2).is_none());

        // Disputing tx 2 should be ignored
        engine.process_transaction(dispute(1, 2));

        let acc = engine.get_account(1);
        // balances unchanged
        assert_eq!(acc.available, decimal(50.0));
        assert_eq!(acc.held, decimal(0.0));
    }

    #[test]
    fn test_duplicate_tx_id_behavior() {
        let mut engine = PaymentEngine::new();

        // First deposit 100 with tx 1
        engine.process_transaction(deposit(1, 1, 100.0));
        // Second deposit with same tx id (duplicate) - will apply again and overwrite recorded tx
        engine.process_transaction(deposit(1, 1, 50.0));

        let acc = engine.get_account(1);
        // Both deposits applied
        assert_eq!(acc.available, decimal(150.0));

        // The recorded transaction should reflect the last inserted amount (50.0)
        let tx = engine.transactions.get(&1).unwrap();
        assert_eq!(tx.amount.round_dp(4), decimal(50.0));
    }

    #[test]
    fn test_resolve_without_dispute_ignored_behaviour() {
        let mut engine = PaymentEngine::new();

        engine.process_transaction(deposit(1, 1, 100.0));
        // Resolve without an active dispute should be ignored
        engine.process_transaction(resolve(1, 1));

        let acc = engine.get_account(1);
        assert_eq!(acc.available, decimal(100.0));
        assert_eq!(acc.held, decimal(0.0));
    }

    #[test]
    fn test_chargeback_without_dispute_ignored_behaviour() {
        let mut engine = PaymentEngine::new();

        engine.process_transaction(deposit(1, 1, 100.0));
        // Chargeback without dispute should be ignored
        engine.process_transaction(chargeback(1, 1));

        let acc = engine.get_account(1);
        assert_eq!(acc.available, decimal(100.0));
        assert_eq!(acc.held, decimal(0.0));
        assert!(!acc.locked);
    }

    #[test]
    fn test_dispute_referencing_withdrawal_moves_available_up_to_amount() {
        let mut engine = PaymentEngine::new();

        engine.process_transaction(deposit(1, 1, 100.0));
        engine.process_transaction(withdrawal(1, 2, 60.0)); // available = 40

        // Dispute the withdrawal tx -> hold up to tx amount (min(amount, available)) i.e., 40
        engine.process_transaction(dispute(1, 2));

        let acc = engine.get_account(1);
        assert_eq!(acc.available, decimal(0.0));
        assert_eq!(acc.held, decimal(40.0));
        // total remains available + held = 40
        assert_eq!(acc.total, decimal(40.0));
    }
}