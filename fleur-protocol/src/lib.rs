//! Fleur Network's independent protocol core.
//!
//! This is a deterministic protocol prototype, not a production consensus
//! implementation. Transactions now require Ed25519 authentication when using
//! apply_signed_transfer; raw apply_transfer remains a low-level state helper.

pub mod evm;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub type Address = String;
pub type Amount = u128;

// Bound startup work and memory when replaying a local journal.
const MAX_CHAIN_JOURNAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_CHAIN_BLOCKS: usize = 100_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub sender: Address,
    pub recipient: Address,
    pub amount: Amount,
    pub fee: Amount,
    pub nonce: u64,
}

impl Transaction {
    /// Stable signing payload: the serialized transaction fields in struct order.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        serde_json::to_vec(self).map_err(|e| ProtocolError::Encoding(e.to_string()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    /// Hex-encoded 32-byte Ed25519 public key.
    pub public_key: String,
    /// Hex-encoded 64-byte Ed25519 signature over Transaction::signing_bytes().
    pub signature: String,
}

impl SignedTransaction {
    pub fn verify(&self) -> Result<(), ProtocolError> {
        let public_key_bytes = decode_hex(&self.public_key)?;
        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| ProtocolError::InvalidPublicKey)?;
        let verifying_key = VerifyingKey::from_bytes(&public_key_array)
            .map_err(|_| ProtocolError::InvalidPublicKey)?;
        let signature_bytes = decode_hex(&self.signature)?;
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| ProtocolError::InvalidSignature)?;
        let expected_address = address_from_public_key(&public_key_array);
        if self.transaction.sender != expected_address {
            return Err(ProtocolError::SignerAddressMismatch);
        }
        verifying_key
            .verify(&self.transaction.signing_bytes()?, &signature)
            .map_err(|_| ProtocolError::InvalidSignature)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub height: u64,
    pub previous_hash: String,
    pub transactions: Vec<SignedTransaction>,
}

impl Block {
    /// Hash a deterministic serialized block. Transaction ordering is significant.
    pub fn hash(&self) -> Result<String, ProtocolError> {
        let bytes = serde_json::to_vec(self).map_err(|e| ProtocolError::Encoding(e.to_string()))?;
        Ok(hex_encode(&Sha256::digest(bytes)))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerState {
    balances: BTreeMap<Address, Amount>,
    nonces: BTreeMap<Address, u64>,
    pub total_fees: Amount,
}

impl LedgerState {
    pub fn with_genesis(balances: impl IntoIterator<Item = (Address, Amount)>) -> Self {
        Self {
            balances: balances.into_iter().collect(),
            nonces: BTreeMap::new(),
            total_fees: 0,
        }
    }

    pub fn balance(&self, address: &str) -> Amount {
        self.balances.get(address).copied().unwrap_or_default()
    }

    pub fn next_nonce(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or_default()
    }

    pub fn apply_signed_transfer(
        &mut self,
        signed: &SignedTransaction,
    ) -> Result<(), ProtocolError> {
        signed.verify()?;
        self.apply_transfer(&signed.transaction)
    }

    /// Low-level state transition. Network/API entry points must use the
    /// signature-checking apply_signed_transfer method instead.
    pub fn apply_transfer(&mut self, tx: &Transaction) -> Result<(), ProtocolError> {
        if tx.sender.is_empty() || tx.recipient.is_empty() {
            return Err(ProtocolError::EmptyAddress);
        }
        if tx.sender == tx.recipient {
            return Err(ProtocolError::SelfTransfer);
        }
        let expected_nonce = self.next_nonce(&tx.sender);
        if tx.nonce != expected_nonce {
            return Err(ProtocolError::BadNonce {
                expected: expected_nonce,
                got: tx.nonce,
            });
        }
        let total = tx
            .amount
            .checked_add(tx.fee)
            .ok_or(ProtocolError::AmountOverflow)?;
        let sender_balance = self.balance(&tx.sender);
        let recipient_balance = self.balance(&tx.recipient);
        let new_sender = sender_balance
            .checked_sub(total)
            .ok_or(ProtocolError::InsufficientFunds)?;
        let new_recipient = recipient_balance
            .checked_add(tx.amount)
            .ok_or(ProtocolError::AmountOverflow)?;
        let new_fees = self
            .total_fees
            .checked_add(tx.fee)
            .ok_or(ProtocolError::AmountOverflow)?;
        let next_nonce = expected_nonce
            .checked_add(1)
            .ok_or(ProtocolError::NonceOverflow)?;

        self.balances.insert(tx.sender.clone(), new_sender);
        self.balances.insert(tx.recipient.clone(), new_recipient);
        self.nonces.insert(tx.sender.clone(), next_nonce);
        self.total_fees = new_fees;
        Ok(())
    }

    /// Apply a block atomically and authenticate every transaction. Invalid
    /// signatures or state transitions leave the original state untouched.
    pub fn apply_block(&mut self, block: &Block) -> Result<(), ProtocolError> {
        let mut candidate = self.clone();
        for tx in &block.transactions {
            candidate.apply_signed_transfer(tx)?;
        }
        *self = candidate;
        Ok(())
    }
}

/// Validates block height and parent linkage before advancing the local chain tip.
/// This is deterministic single-node chain validation, not distributed consensus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blockchain {
    pub ledger: LedgerState,
    pub tip_height: u64,
    pub tip_hash: String,
}

impl Blockchain {
    pub fn new(genesis_state: LedgerState, genesis_hash: impl Into<String>) -> Self {
        Self {
            ledger: genesis_state,
            tip_height: 0,
            tip_hash: genesis_hash.into(),
        }
    }

    pub fn apply_block(&mut self, block: &Block) -> Result<String, ProtocolError> {
        let expected_height = self
            .tip_height
            .checked_add(1)
            .ok_or(ProtocolError::BlockHeightOverflow)?;
        if block.height != expected_height {
            return Err(ProtocolError::BadBlockHeight {
                expected: expected_height,
                got: block.height,
            });
        }
        if block.previous_hash != self.tip_hash {
            return Err(ProtocolError::BadParentHash);
        }
        let block_hash = block.hash()?;
        let mut next_ledger = self.ledger.clone();
        next_ledger.apply_block(block)?;
        self.ledger = next_ledger;
        self.tip_height = block.height;
        self.tip_hash = block_hash.clone();
        Ok(block_hash)
    }
}

/// Append-only block journal. Opening a journal replays and validates every
/// block from the supplied genesis state. This is a local persistence layer,
/// not a database or a distributed consensus mechanism.
pub struct PersistentChain {
    pub chain: Blockchain,
    journal_path: PathBuf,
}

impl PersistentChain {
    pub fn open(
        journal_path: impl AsRef<Path>,
        genesis_state: LedgerState,
        genesis_hash: impl Into<String>,
    ) -> Result<Self, ProtocolError> {
        let journal_path = journal_path.as_ref().to_path_buf();
        let mut chain = Blockchain::new(genesis_state, genesis_hash);
        let journal = match fs::read_to_string(&journal_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(ProtocolError::Io(error.to_string())),
        };

        if journal.len() > MAX_CHAIN_JOURNAL_BYTES {
            return Err(ProtocolError::JournalTooLarge {
                limit: MAX_CHAIN_JOURNAL_BYTES,
                actual: journal.len(),
            });
        }

        let mut replayed_blocks = 0usize;
        for (line_index, line) in journal.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if replayed_blocks >= MAX_CHAIN_BLOCKS {
                return Err(ProtocolError::TooManyJournalBlocks {
                    limit: MAX_CHAIN_BLOCKS,
                });
            }
            let block: Block =
                serde_json::from_str(line).map_err(|error| ProtocolError::InvalidJournal {
                    line: line_index + 1,
                    message: error.to_string(),
                })?;
            chain.apply_block(&block)?;
            replayed_blocks += 1;
        }

        Ok(Self {
            chain,
            journal_path,
        })
    }

    /// Validate against a cloned chain first, append and sync the block, then
    /// publish the new in-memory tip. A rejected block never reaches the journal.
    pub fn append_block(&mut self, block: &Block) -> Result<String, ProtocolError> {
        let mut next_chain = self.chain.clone();
        let hash = next_chain.apply_block(block)?;
        if let Some(parent) = self
            .journal_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| ProtocolError::Io(error.to_string()))?;
        }
        let encoded = serde_json::to_string(block)
            .map_err(|error| ProtocolError::Encoding(error.to_string()))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)
            .map_err(|error| ProtocolError::Io(error.to_string()))?;
        writeln!(file, "{encoded}").map_err(|error| ProtocolError::Io(error.to_string()))?;
        file.sync_all()
            .map_err(|error| ProtocolError::Io(error.to_string()))?;
        self.chain = next_chain;
        Ok(hash)
    }
}

pub fn address_from_public_key(public_key: &[u8; 32]) -> Address {
    format!("flr_{}", hex_encode(public_key))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(input: &str) -> Result<Vec<u8>, ProtocolError> {
    if !input.len().is_multiple_of(2) {
        return Err(ProtocolError::InvalidHex);
    }
    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&input[i..i + 2], 16).map_err(|_| ProtocolError::InvalidHex))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    EmptyAddress,
    SelfTransfer,
    BadNonce { expected: u64, got: u64 },
    InsufficientFunds,
    AmountOverflow,
    NonceOverflow,
    Encoding(String),
    InvalidHex,
    InvalidPublicKey,
    InvalidSignature,
    SignerAddressMismatch,
    BadBlockHeight { expected: u64, got: u64 },
    BadParentHash,
    BlockHeightOverflow,
    Io(String),
    InvalidJournal { line: usize, message: String },
    JournalTooLarge { limit: usize, actual: usize },
    TooManyJournalBlocks { limit: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_transfer(nonce: u64, amount: u128, fee: u128) -> SignedTransaction {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let tx = Transaction {
            sender: address_from_public_key(&public_key),
            recipient: "flr_recipient".into(),
            amount,
            fee,
            nonce,
        };
        let signature = signing_key.sign(&tx.signing_bytes().unwrap());
        SignedTransaction {
            transaction: tx,
            public_key: hex_encode(&public_key),
            signature: hex_encode(&signature.to_bytes()),
        }
    }

    fn transfer(nonce: u64, amount: u128, fee: u128) -> Transaction {
        Transaction {
            sender: "alice".into(),
            recipient: "bob".into(),
            amount,
            fee,
            nonce,
        }
    }

    #[test]
    fn valid_signed_transfer_moves_balance_and_collects_fee() {
        let signed = signed_transfer(0, 30, 2);
        let mut state = LedgerState::with_genesis([
            (signed.transaction.sender.clone(), 100),
            ("flr_recipient".into(), 5),
        ]);
        state.apply_signed_transfer(&signed).unwrap();
        assert_eq!(state.balance(&signed.transaction.sender), 68);
        assert_eq!(state.balance("flr_recipient"), 35);
        assert_eq!(state.total_fees, 2);
        assert_eq!(state.next_nonce(&signed.transaction.sender), 1);
    }

    #[test]
    fn tampered_transaction_is_rejected_without_mutation() {
        let mut signed = signed_transfer(0, 30, 2);
        signed.transaction.amount = 31;
        let mut state = LedgerState::with_genesis([(signed.transaction.sender.clone(), 100)]);
        let before = state.clone();
        assert_eq!(
            state.apply_signed_transfer(&signed),
            Err(ProtocolError::InvalidSignature)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn signature_from_wrong_address_is_rejected() {
        let mut signed = signed_transfer(0, 30, 2);
        signed.transaction.sender = "flr_someone_else".into();
        let mut state = LedgerState::with_genesis([("flr_someone_else".into(), 100)]);
        assert_eq!(
            state.apply_signed_transfer(&signed),
            Err(ProtocolError::SignerAddressMismatch)
        );
    }

    #[test]
    fn replayed_nonce_is_rejected_without_mutation() {
        let signed = signed_transfer(0, 10, 1);
        let mut state = LedgerState::with_genesis([(signed.transaction.sender.clone(), 100)]);
        state.apply_signed_transfer(&signed).unwrap();
        let before = state.clone();
        assert_eq!(
            state.apply_signed_transfer(&signed),
            Err(ProtocolError::BadNonce {
                expected: 1,
                got: 0,
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn invalid_late_transaction_rolls_back_entire_block() {
        let first = signed_transfer(0, 10, 1);
        let sender = first.transaction.sender.clone();
        let second = signed_transfer(1, 10_000, 1);
        let mut state = LedgerState::with_genesis([(sender, 100)]);
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![first, second],
        };
        let before = state.clone();
        assert_eq!(
            state.apply_block(&block),
            Err(ProtocolError::InsufficientFunds)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn block_hash_is_deterministic() {
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![signed_transfer(0, 7, 1)],
        };
        assert_eq!(block.hash().unwrap(), block.hash().unwrap());
        assert_eq!(block.hash().unwrap().len(), 64);
    }

    #[test]
    fn blockchain_rejects_wrong_parent_and_keeps_tip_unchanged() {
        let signed = signed_transfer(0, 7, 1);
        let sender = signed.transaction.sender.clone();
        let ledger = LedgerState::with_genesis([(sender, 100)]);
        let mut chain = Blockchain::new(ledger, "genesis");
        let block = Block {
            height: 1,
            previous_hash: "wrong-parent".into(),
            transactions: vec![signed],
        };
        let before = chain.clone();
        assert_eq!(chain.apply_block(&block), Err(ProtocolError::BadParentHash));
        assert_eq!(chain, before);
    }

    #[test]
    fn blockchain_advances_tip_after_valid_block() {
        let signed = signed_transfer(0, 7, 1);
        let sender = signed.transaction.sender.clone();
        let ledger = LedgerState::with_genesis([(sender, 100)]);
        let mut chain = Blockchain::new(ledger, "genesis");
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![signed],
        };
        let hash = chain.apply_block(&block).unwrap();
        assert_eq!(chain.tip_height, 1);
        assert_eq!(chain.tip_hash, hash);
    }

    #[test]
    fn persistent_journal_replays_validated_chain_state() {
        let path =
            std::env::temp_dir().join(format!("fleur-chain-journal-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let signed = signed_transfer(0, 17, 2);
        let sender = signed.transaction.sender.clone();
        let genesis = LedgerState::with_genesis([(sender.clone(), 100)]);
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![signed],
        };
        {
            let mut persistent = PersistentChain::open(&path, genesis.clone(), "genesis").unwrap();
            persistent.append_block(&block).unwrap();
            assert_eq!(persistent.chain.ledger.balance(&sender), 81);
        }
        let reopened = PersistentChain::open(&path, genesis, "genesis").unwrap();
        assert_eq!(reopened.chain.tip_height, 1);
        assert_eq!(reopened.chain.ledger.balance(&sender), 81);
        assert_eq!(reopened.chain.ledger.total_fees, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejected_persistent_block_is_not_written_to_journal() {
        let path =
            std::env::temp_dir().join(format!("fleur-rejected-block-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let signed = signed_transfer(0, 7, 1);
        let sender = signed.transaction.sender.clone();
        let genesis = LedgerState::with_genesis([(sender, 100)]);
        let invalid_block = Block {
            height: 1,
            previous_hash: "wrong-parent".into(),
            transactions: vec![signed],
        };

        let mut persistent =
            PersistentChain::open(&path, genesis, "genesis").expect("open clean chain journal");
        let before = persistent.chain.clone();
        assert_eq!(
            persistent.append_block(&invalid_block),
            Err(ProtocolError::BadParentHash)
        );
        assert_eq!(persistent.chain, before);
        assert!(
            !path.exists(),
            "a rejected block must not create or append to the canonical journal"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn opening_corrupt_canonical_journal_fails_closed() {
        let path = std::env::temp_dir().join(format!(
            "fleur-corrupt-chain-journal-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let corrupted = "{invalid block json}\n";
        std::fs::write(&path, corrupted).expect("write corrupted canonical journal");

        let result = PersistentChain::open(&path, LedgerState::default(), "genesis");
        assert!(
            matches!(result, Err(ProtocolError::InvalidJournal { line: 1, .. })),
            "startup must reject a malformed persisted block instead of silently resetting"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("read canonical journal"),
            corrupted,
            "startup validation must not rewrite the existing journal"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn oversized_canonical_journal_fails_closed_without_modification() {
        let path = std::env::temp_dir().join(format!(
            "fleur-oversized-chain-journal-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let oversized = vec![b'x'; MAX_CHAIN_JOURNAL_BYTES + 1];
        std::fs::write(&path, &oversized).expect("write oversized journal");

        let result = PersistentChain::open(&path, LedgerState::default(), "genesis");
        assert_eq!(
            result.err(),
            Some(ProtocolError::JournalTooLarge {
                limit: MAX_CHAIN_JOURNAL_BYTES,
                actual: oversized.len(),
            })
        );
        assert_eq!(
            std::fs::read(&path).expect("read original journal"),
            oversized,
            "startup must not rewrite an oversized journal"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn too_many_canonical_journal_blocks_are_rejected() {
        let path = std::env::temp_dir().join(format!(
            "fleur-too-many-chain-blocks-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        // The journal must contain valid sequential blocks so replay reaches
        // the configured block-count limit instead of failing on an earlier
        // height or parent-hash error.
        let mut previous_hash = "genesis".to_string();
        let mut journal = String::new();
        for height in 1..=MAX_CHAIN_BLOCKS + 1 {
            let block = Block {
                height: height as u64,
                previous_hash: previous_hash.clone(),
                transactions: Vec::new(),
            };
            previous_hash = block.hash().expect("hash generated block");
            if !journal.is_empty() {
                journal.push('\n');
            }
            journal.push_str(&serde_json::to_string(&block).expect("encode generated block"));
        }
        std::fs::write(&path, &journal).expect("write too many blocks");

        let result = PersistentChain::open(&path, LedgerState::default(), "genesis");
        assert_eq!(
            result.err(),
            Some(ProtocolError::TooManyJournalBlocks {
                limit: MAX_CHAIN_BLOCKS,
            })
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn arithmetic_overflow_is_rejected_without_mutation() {
        let mut state = LedgerState::with_genesis([("alice".into(), u128::MAX)]);
        let before = state.clone();
        let tx = transfer(0, u128::MAX, 1);
        assert_eq!(
            state.apply_transfer(&tx),
            Err(ProtocolError::AmountOverflow)
        );
        assert_eq!(state, before);
    }
}
