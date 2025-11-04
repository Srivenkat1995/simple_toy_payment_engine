mod orchestrator;
mod transactions;
mod engine;
mod accounts;

use::std::env;
use::std::process;

use orchestrator::run;
use env_logger;
use log::info;

fn main() {
    // Collect command-line arguments - expecting exactly one argument for the CSV file path
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <transactions.csv>", args[0]);
        process::exit(1);
    }
    // Call the run function with the provided filename
    let filename = &args[1];
    // Initialize logger (respect RUST_LOG env var if set)
    env_logger::init();

    info!("starting payments engine with file: {}", filename);

    if let Err(e) = run(filename) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
