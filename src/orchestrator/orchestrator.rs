use std::fs::File;
use std::error::Error;
use csv::ReaderBuilder;

use crate::engine::PaymentEngine;
use crate::transactions::TransactionRecord;

pub fn run(filename: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr: csv::Reader<File> = ReaderBuilder::new().trim(csv::Trim::All).from_reader(file);

    let mut engine = PaymentEngine::new();

    for result in rdr.deserialize() {
        let record: TransactionRecord = result?;
        engine.process_transaction(record);
    }

    engine.output_accounts();

    Ok(())
}