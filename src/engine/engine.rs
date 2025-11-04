use std::collections::HashMap;
use csv::Writer;
use rust_decimal::prelude::FromPrimitive;

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
                    account.withdraw(amount);

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
                            }
                        }
                        TransactionType::Chargeback => {
                            if tx.disputed {
                                account.chargeback(amount);
                                tx.disputed = false;
                            }
                        }
                        _ => {}
                    }

                    // Put the transaction back into the map
                    self.transactions.insert(record.tx, tx);
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
}