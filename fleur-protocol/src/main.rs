use ed25519_dalek::{Signer, SigningKey};
use fleur_protocol::{address_from_public_key, Block, LedgerState, SignedTransaction, Transaction};

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    // Deterministic demo key only; never use this key pattern for real funds.
    let signing_key = SigningKey::from_bytes(&[9u8; 32]);
    let public_key = signing_key.verifying_key().to_bytes();
    let alice = address_from_public_key(&public_key);
    let bob = "flr_demo_recipient".to_string();

    let tx = Transaction {
        sender: alice.clone(),
        recipient: bob.clone(),
        amount: 100,
        fee: 1,
        nonce: 0,
    };
    let signature = signing_key.sign(&tx.signing_bytes().expect("encode transaction"));
    let signed_tx = SignedTransaction {
        transaction: tx,
        public_key: hex_encode(&public_key),
        signature: hex_encode(&signature.to_bytes()),
    };

    let mut state = LedgerState::with_genesis([(alice.clone(), 1_000)]);
    let block = Block {
        height: 1,
        previous_hash: "GENESIS".into(),
        transactions: vec![signed_tx],
    };
    state
        .apply_block(&block)
        .expect("signed demo block should be valid");

    println!("Fleur Network protocol scaffold");
    println!("height: {}", block.height);
    println!("block_hash: {}", block.hash().expect("block serialization"));
    println!("alice_address: {alice}");
    println!("alice_balance: {}", state.balance(&alice));
    println!("bob_balance: {}", state.balance(&bob));
    println!("fees_collected: {}", state.total_fees);
    println!("NOTE: deterministic demo key only; consensus and networking are not implemented.");
}
