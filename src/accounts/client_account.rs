use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

#[derive(Debug)]
pub struct ClientAccount {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl ClientAccount {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Decimal::from_f64(0.0).unwrap(),
            held: Decimal::from_f64(0.0).unwrap(),
            total: Decimal::from_f64(0.0).unwrap(),
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Decimal) {
        if self.locked {
            return;
        }
        self.available += amount;
        self.total += amount;
    }

    pub fn withdraw(&mut self, amount: Decimal) -> bool {
        if self.locked || self.available < amount {
            return false;
        }
        self.available -= amount;
        self.total -= amount;
        true
    }

    pub fn hold(&mut self, amount: Decimal) {
        if self.locked {
            return;
        }
        // Only hold what is actually available
        let hold_amount = amount.min(self.available);
        self.available -= hold_amount;
        self.held += hold_amount;
    }

    pub fn release(&mut self, amount: Decimal) {
        if self.locked {
            return;
        }
        // Only release up to what is held
        let release_amount = amount.min(self.held);
        self.held -= release_amount;
        self.available += release_amount;
    }

    pub fn chargeback(&mut self, amount: Decimal) {
        if self.locked {
            return;
        }
        // Only chargeback up to what is held
        let cb_amount = amount.min(self.held);
        self.held -= cb_amount;
        self.total -= cb_amount;
        self.locked = true;
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    fn decimal(amount: f64) -> Decimal {
        Decimal::from_f64(amount).unwrap()
    }

    #[test]
    fn test_deposit() {
        let mut acc = ClientAccount::new(1);
        acc.deposit(decimal(100.0));
        assert_eq!(acc.available, decimal(100.0));
        assert_eq!(acc.held, decimal(0.0));
        assert_eq!(acc.total, decimal(100.0));
        assert!(!acc.locked);
    }

    #[test]
    fn test_withdraw() {
        let mut acc = ClientAccount::new(1);
        acc.deposit(decimal(100.0));
        acc.withdraw(decimal(40.0));
        assert_eq!(acc.available, decimal(60.0));
        assert_eq!(acc.total, decimal(60.0));

        // Withdraw more than available → no change
        acc.withdraw(decimal(100.0));
        assert_eq!(acc.available, decimal(60.0));
        assert_eq!(acc.total, decimal(60.0));
    }

    #[test]
    fn test_hold_and_release() {
        let mut acc = ClientAccount::new(1);
        acc.deposit(decimal(100.0));

        // Hold 70
        acc.hold(decimal(70.0));
        assert_eq!(acc.available, decimal(30.0));
        assert_eq!(acc.held, decimal(70.0));

        // Release 50
        acc.release(decimal(50.0));
        assert_eq!(acc.available, decimal(80.0));
        assert_eq!(acc.held, decimal(20.0));
    }

    #[test]
    fn test_chargeback_locks_account() {
        let mut acc = ClientAccount::new(1);
        acc.deposit(decimal(100.0));
        acc.hold(decimal(50.0));
        acc.chargeback(decimal(50.0));

        assert_eq!(acc.available, decimal(50.0));
        assert_eq!(acc.held, decimal(0.0));
        assert_eq!(acc.total, decimal(50.0));
        assert!(acc.locked);

        // Cannot deposit or withdraw after lock
        acc.deposit(decimal(10.0));
        acc.withdraw(decimal(10.0));
        assert_eq!(acc.available, decimal(50.0));
        assert_eq!(acc.total, decimal(50.0));
    }
}
