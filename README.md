# Simple Toy Payments Engine

A Rust-based payments engine that processes CSV transactions and maintains client account balances.  
Supports **Deposits, Withdrawals, Disputes, Resolves, and Chargebacks**.

---

## Features

- Process transactions from a CSV file.
- Track client balances:
  - `available` – funds available for withdrawal or trading.
  - `held` – funds held due to disputes.
  - `total` – sum of `available` + `held`.
  - `locked` – account frozen after chargebacks.
- Maintain chronological transaction order.
- Handle disputes, resolves, and chargebacks correctly.
- Decimal precision up to **4 digits** to avoid floating-point errors.
- Exposes a **CLI interface** for batch processing.

---

## Usage

```bash
cargo run -- transactions.csv > accounts.csv
```

- transactions.csv – Input CSV file containing transactions.
- accounts.csv – Output CSV file with final account balances.

## Implementation Summary

- The payments engine is implemented in an Object-Oriented style:

- ClientAccount – Stores client balances and locked status.

- TransactionRecord – Represents a transaction from the CSV.

- Engine – Processes transactions, updates accounts.

- Orchestrator – Reads CSV input, sends transactions to the engine, writes CSV output.


### UML Class Diagram (Mermaid)

```mermaid
classDiagram
    class ClientAccount {
        +u16 client
        +Decimal available
        +Decimal held
        +Decimal total
        +bool locked
        +new(client: u16)
        +deposit(amount: Decimal)
        +withdraw(amount: Decimal)
        +hold(amount: Decimal)
        +release(amount: Decimal)
        +chargeback(amount: Decimal)
    }

    class TransactionRecord {
        +TransactionType tx_type
        +u16 client
        +u32 tx
        +Option~Decimal~ amount
    }

    class Engine {
        -HashMap~u16, ClientAccount~ accounts
        -HashMap~u32, TransactionRecord~ transactions
        +process_transaction(tx: TransactionRecord)
        +get_accounts() : Vec~ClientAccount~
    }

    class Orchestrator {
        +run(input_file: &str)
    }

    ClientAccount <|-- Engine : owns
    Engine --> Orchestrator : used by
    TransactionRecord --> Engine : processed by
