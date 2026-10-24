use fleur_protocol::{Block, LedgerState, Transaction};

fn main() {
    let mut state = LedgerState::with_genesis([("fleur-genesis-alice".to_string(), 1_000)]);
    let tx = Transaction {
        sender: "fleur-genesis-alice".into(),
        recipient: "fleur-genesis-bob".into(),
        amount: 100,
        fee: 1,
        nonce: 0,
    };
    state.apply_transfer(&tx).expect("demo transfer should be valid");
    let block = Block {
        height: 1,
        previous_hash: "GENESIS".into(),
        transactions: vec![tx],
    };
    println!("Fleur Network protocol scaffold");
    println!("height: {}", block.height);
    println!("block_hash: {}", block.hash().expect("block serialization"));
    println!("alice_balance: {}", state.balance("fleur-genesis-alice"));
    println!("bob_balance: {}", state.balance("fleur-genesis-bob"));
    println!("fees_collected: {}", state.total_fees);
    println!("NOTE: local demo only; no signature verification or consensus yet.");
}
