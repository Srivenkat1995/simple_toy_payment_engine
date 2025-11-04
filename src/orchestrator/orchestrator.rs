use std::fs::File;
use std::error::Error;
use csv::ReaderBuilder;

use crate::transactions::TransactionRecord;

pub fn run(filename: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr: csv::Reader<File> = ReaderBuilder::new().trim(csv::Trim::All).from_reader(file);

    for result in rdr.deserialize() {
        let record: TransactionRecord = result?;
        println!("{:?}", record);
    }

    Ok(())
}