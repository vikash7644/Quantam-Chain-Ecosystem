# Fleur Network — independent protocol

Fleur Network is an independent blockchain protocol project. It is not a
Solana fork and shares no source files with Zenvora Lang. The selected smart
contract target is Solidity/EVM only; an isolated REVM execution smoke test exists, but EVM is not integrated into chain state transitions or RPC yet.

## Implemented in this development slice

- Deterministic transfer ledger with checked arithmetic and fee accounting.
- Ed25519 signatures with public-key-to-sender binding and tamper rejection.
- Per-sender sequential nonces to reject replayed transactions.
- Atomic block application that rejects invalid transactions without partial state changes.
- SHA-256 block hashes, block-height checks, and parent-hash continuity checks.
- Append-only JSONL journal with validation and deterministic replay on restart.
- Single-node HTTP JSON-RPC methods: `fleur_chainStatus`, `fleur_getBalance`,
  and `fleur_submitTransaction`.
- A pinned REVM 43.0.3 smoke test that deploys bytecode and verifies contract storage execution in an isolated in-memory EVM.
- GitHub Actions checks for formatting, tests, and a release build.

## Not public-testnet ready

This is a development scaffold, **not a public testnet**. There is no distributed
consensus or peer-to-peer networking. REVM currently runs only in an isolated test;
Fleur RPC does not yet expose Solidity contract deployment/calls, and EVM state is
not connected to the persistent ledger. The JSONL journal is an MVP persistence layer, not a
production database. RPC transaction submission currently seals one accepted
transaction per block; there is no transaction mempool or production ordering.
The demo binary uses a deterministic test key and must never be used with real
funds. Do not expose the development RPC server publicly.

## Clean-environment bootstrap

Install a current stable Rust toolchain with Cargo, then run from the repository
root:

```sh
cd fleur-protocol
cargo fmt --all -- --check
cargo test --all-targets
cargo build --release
```

Run the demo binary:

```sh
cargo run --release
```

Run the single-node development JSON-RPC server in a separate process:

```sh
FLEUR_RPC_ADDR=127.0.0.1:8545 cargo run --release --bin node
```

Optional environment variables:

- `FLEUR_JOURNAL`: append-only journal path (default `./data/fleur-chain.jsonl`).
- `FLEUR_GENESIS_ADDRESS`: address to fund at genesis (default empty ledger).
- `FLEUR_GENESIS_BALANCE`: initial balance for that address (default `0`).
- `FLEUR_RPC_ADDR`: bind address (default `127.0.0.1:8545`).

The server supports JSON-RPC 2.0 over `POST /rpc`. For example, query status:

```sh
curl -s http://127.0.0.1:8545/rpc \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"fleur_chainStatus"}'
```

## Ten-day sprint target

1. Protocol rules and clean bootstrap.
2. Signed transaction envelope and key verification.
3. Block validation and parent continuity.
4. Append-only journal and replay validation.
5. Single-node RPC.
6. Integrate a real EVM engine and add compatibility tests.
7. Add Solidity contract deploy/call path.
8. Multi-node synchronization and adversarial tests.
9. End-to-end testnet scripts and explorer-facing APIs.
10. CI evidence, operational docs, and limitations report.

The sprint target is a testnet prototype, not a security-audited mainnet.
