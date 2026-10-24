//! Fleur Network's independent protocol core.
//!
//! This first sprint slice is intentionally a deterministic local ledger model,
//! not a production consensus implementation. Transaction authentication and
//! network consensus are explicit follow-up milestones before any public testnet.

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub height: u64,
    pub previous_hash: String,
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Hash a canonical JSON representation. BTreeMap is used for state maps;
    /// block transaction order is preserved and therefore consensus-relevant.
    pub fn hash(&self) -> Result<String, ProtocolError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|e| ProtocolError::Encoding(e.to_string()))?;
        let digest = Sha256::digest(bytes);
        Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
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

    /// Apply one transfer atomically: all arithmetic and validation happen
    /// before any state is mutated. This does not authenticate the sender yet.
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
        let total = tx.amount.checked_add(tx.fee).ok_or(ProtocolError::AmountOverflow)?;
        let sender_balance = self.balance(&tx.sender);
        let recipient_balance = self.balance(&tx.recipient);
        let new_sender = sender_balance.checked_sub(total).ok_or(ProtocolError::InsufficientFunds)?;
        let new_recipient = recipient_balance.checked_add(tx.amount).ok_or(ProtocolError::AmountOverflow)?;
        let new_fees = self.total_fees.checked_add(tx.fee).ok_or(ProtocolError::AmountOverflow)?;
        let next_nonce = expected_nonce.checked_add(1).ok_or(ProtocolError::NonceOverflow)?;

        self.balances.insert(tx.sender.clone(), new_sender);
        self.balances.insert(tx.recipient.clone(), new_recipient);
        self.nonces.insert(tx.sender.clone(), next_nonce);
        self.total_fees = new_fees;
        Ok(())
    }

    pub fn apply_block(&mut self, block: &Block) -> Result<(), ProtocolError> {
        // Apply to a clone so a later invalid transaction cannot partially
        // mutate the caller's state.
        let mut candidate = self.clone();
        for tx in &block.transactions {
            candidate.apply_transfer(tx)?;
        }
        *self = candidate;
        Ok(())
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn valid_transfer_moves_balance_and_collects_fee() {
        let mut state = LedgerState::with_genesis([("alice".into(), 100), ("bob".into(), 5)]);
        state.apply_transfer(&transfer(0, 30, 2)).unwrap();
        assert_eq!(state.balance("alice"), 68);
        assert_eq!(state.balance("bob"), 35);
        assert_eq!(state.total_fees, 2);
        assert_eq!(state.next_nonce("alice"), 1);
    }

    #[test]
    fn replayed_nonce_is_rejected_without_mutation() {
        let mut state = LedgerState::with_genesis([("alice".into(), 100)]);
        state.apply_transfer(&transfer(0, 10, 1)).unwrap();
        let before = state.clone();
        assert_eq!(
            state.apply_transfer(&transfer(0, 10, 1)),
            Err(ProtocolError::BadNonce { expected: 1, got: 0 })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn invalid_late_transaction_rolls_back_entire_block() {
        let mut state = LedgerState::with_genesis([("alice".into(), 100)]);
        let block = Block {
            height: 1,
            previous_hash: "genesis".into(),
            transactions: vec![transfer(0, 10, 1), transfer(1, 10_000, 1)],
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
            transactions: vec![transfer(0, 7, 1)],
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
