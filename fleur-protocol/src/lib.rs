//! Fleur Network's independent protocol core.
//!
//! This is a deterministic protocol prototype, not a production consensus
//! implementation. Transactions now require Ed25519 authentication when using
//! apply_signed_transfer; raw apply_transfer remains a low-level state helper.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub type Address = String;
pub type Amount = u128;

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
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| ProtocolError::InvalidSignature)?;
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
        let bytes =
            serde_json::to_vec(self).map_err(|e| ProtocolError::Encoding(e.to_string()))?;
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
        let next_nonce = expected_nonce.checked_add(1).ok_or(ProtocolError::NonceOverflow)?;

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

pub fn address_from_public_key(public_key: &[u8; 32]) -> Address {
    format!("flr_{}", hex_encode(public_key))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(input: &str) -> Result<Vec<u8>, ProtocolError> {
    if input.len() % 2 != 0 {
        return Err(ProtocolError::InvalidHex);
    }
    (0..input.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&input[i..i + 2], 16).map_err(|_| ProtocolError::InvalidHex)
        })
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
        let mut state = LedgerState::with_genesis([("alice".into(), 100)]);
        let first = signed_transfer(0, 10, 1);
        let second = signed_transfer(1, 10_000, 1);
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![first, second],
        };
        let before = state.clone();
        assert_eq!(state.apply_block(&block), Err(ProtocolError::InsufficientFunds));
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
    fn arithmetic_overflow_is_rejected_without_mutation() {
        let mut state = LedgerState::with_genesis([("alice".into(), u128::MAX)]);
        let before = state.clone();
        let tx = transfer(0, u128::MAX, 1);
        assert_eq!(state.apply_transfer(&tx), Err(ProtocolError::AmountOverflow));
        assert_eq!(state, before);
    }
}
