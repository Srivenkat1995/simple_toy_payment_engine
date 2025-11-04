use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub client: u16,
    pub amount: Decimal,
    pub disputed: bool,
}
