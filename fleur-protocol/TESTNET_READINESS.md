# Fleur Network Testnet Readiness Gates

This checklist is intentionally conservative. A passing unit-test suite is not proof that the network is ready for public funds or public validators.

## Current development baseline

- [x] Deterministic single-node ledger with checked transfer arithmetic and fee accounting.
- [x] Signed Ed25519 transfer verification and sender/address binding.
- [x] Sender nonce replay protection.
- [x] Block height/parent continuity checks and append-only block journal replay.
- [x] Single-node JSON-RPC for chain status, balance, and signed transfer submission.
- [x] Native coin metadata RPC identifies BVRX and reports genesis-configuration-defined issuance; no minting RPC is exposed.
- [x] Genesis-issued supply tracking, checked aggregate genesis arithmetic, circulating-supply reporting, and supply invariant tests.
- [ ] Explicit maximum supply cap and validator fee-reward policy finalized.
- [x] Operator `GET /health` endpoint with a smoke-test check.
- [x] REVM sandbox and replayable EVM command journal smoke-tested in CI.
- [ ] EVM contract state committed atomically with chain blocks.
- [ ] EVM transaction signatures, account nonce, intrinsic gas, fee charging/refunds, and failure semantics specified and tested.
- [x] Existing deployed-contract call path that replays persisted state changes across calls/restarts (still separate from canonical blocks).
- [ ] Mempool policy, deterministic transaction ordering, and block production rules.
- [ ] Multi-node peer discovery, authenticated peer messages, block propagation, and synchronization.
- [ ] A consensus/finality mechanism with adversarial and restart tests.
- [ ] Public-facing wallet connected to real chain RPC with transaction signing and receipt/status handling.
- [ ] Explorer API/UI driven by canonical blocks, transactions, balances, and contract data.
- [ ] Clean-environment node bootstrap, configuration validation, backups, and operator documentation.
- [ ] End-to-end multi-node testnet deployment and external test instructions.
- [ ] Independent security review and documented incident/recovery process before any real-value use.

## Release gates

### Gate A — single-node correctness
Run `sh scripts/verify.sh` and `sh scripts/evm-smoke.sh` from a clean checkout. Add restart tests for every persisted state transition. Invalid transactions and failed contract executions must not partially change canonical state.

### Gate B — EVM and ledger atomicity
A contract transaction must be validated and executed against the parent block state. Its EVM state diff, sender nonce, gas charge/refund, receipt, and block hash must commit together or all roll back. The separate JSONL EVM command journal does not satisfy this gate.

### Gate C — multi-node correctness
At least three independently started nodes must converge on the same finalized block/state under normal operation, restarts, delayed messages, and invalid peer data. Document the consensus and threat model; a shared file or one process is not consensus.

### Gate D — wallet and explorer
Wallet balances and receipts must be read from canonical node state. Signed transfers must be verified by the node, have clear confirmation status, and be tested against rejected/duplicate transactions. Explorer data must be derived from the same canonical chain.

### Gate E — public testnet
Publish reproducible node setup, network identifiers, genesis/configuration hashes, RPC limits, faucet policy (if any), monitoring, upgrade/recovery instructions, and a known-limitations page. Keep real funds disabled until security review is complete.

## Current status

**Development prototype only. Not public-testnet ready.** Current CI validates the existing single-node slice and the EVM replay smoke test; it does not validate consensus, multi-node behavior, or atomic EVM/ledger integration.
