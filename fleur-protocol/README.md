# Fleur Network — independent protocol

Fleur Network is a new independent blockchain protocol project. It is not a
Solana fork and does not share source files with Zenvora Lang. The contract
target is Solidity/EVM compatibility only.

## Current sprint slice

- Deterministic transfer ledger with checked arithmetic.
- Ed25519 public-key signature verification and sender-address binding.
- Per-sender sequential nonces to reject replayed transactions.
- Transaction fees accounted for separately.
- Atomic block application that verifies signatures and rolls back on invalid transactions.
- SHA-256 block hashing over serialized signed block contents.
- GitHub Actions workflow for formatting, tests, and release compilation.

## Explicitly not testnet-ready yet

This slice is **not** a public testnet. The local signed-transfer path verifies
Ed25519 signatures, but block height/parent continuity is not yet enforced;
there is no consensus or peer-to-peer networking, and persistent storage,
RPC, and EVM execution are not implemented. The CLI uses a deterministic demo
private key and must never be used with real funds. Do not use real funds or expose this as a
production node.

## Local development

Install Rust stable, then:

```sh
cd fleur-protocol
cargo fmt --all -- --check
cargo test --all-targets
cargo run --release
```

## Ten-day target

1. Protocol rules and reproducible build.
2. ~~Signed transaction envelope and key verification~~ (implemented in the local protocol crate; CI still needs to verify).
3. Block validation and chain continuity.
4. Persistent state and deterministic replay.
5. Node API and local devnet configuration.
6. EVM execution design and compatibility tests.
7. Solidity contract deploy/call path (scoped to achievable MVP).
8. Multi-node sync and adversarial tests.
9. End-to-end testnet scripts and explorer-facing APIs.
10. Clean-environment bootstrap, CI evidence, and known-limitations report.

The ten-day goal is a testnet prototype, not a security-audited mainnet.
