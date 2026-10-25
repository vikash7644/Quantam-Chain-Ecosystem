# Fleur Network — independent protocol

Fleur Network is an independent blockchain protocol project. It is not a
Solana fork and shares no source files with Zenvora Lang. The selected smart
contract target is Solidity/EVM only. A REVM-backed sandbox method is exposed over RPC,
but it does not share state with the persistent ledger.

## Implemented in this development slice

- Deterministic transfer ledger with checked arithmetic and fee accounting.
- Ed25519 signatures with public-key-to-sender binding and tamper rejection.
- Per-sender sequential nonces to reject replayed transactions.
- Atomic block application that rejects invalid transactions without partial state changes.
- SHA-256 block hashes, block-height checks, and parent-hash continuity checks.
- Append-only JSONL journal with validation and deterministic replay on restart.
- Single-node HTTP JSON-RPC methods: `fleur_chainStatus`, `fleur_getBalance`,
  and `fleur_submitTransaction`.
- A pinned REVM 43.0.3 execution sandbox plus a replayable EVM command journal (`fleur_evmPersistentDeployAndCall`) that reconstructs prior deployments and calls from JSONL on each request/restart.
- GitHub Actions checks for formatting, tests, and a release build.

## Not public-testnet ready

This is a development scaffold, **not a public testnet**. There is no distributed
consensus or peer-to-peer networking. `fleur_evmSandboxDeployAndCall` runs init bytecode
and calls the resulting contract in a fresh in-memory EVM, and is not a production Ethereum-compatible transaction path. The separate EVM command journal can replay prior deployments/calls across requests and restarts, but it is not included in chain blocks, has no dedicated state query/call-existing-contract RPC yet, and is not an atomic part of block validation. Ethereum account/signature handling
and EVM gas settlement are not integrated with ledger transactions. The JSONL journal is an MVP persistence layer, not a
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

A one-command verification script is also available:

```sh
sh scripts/verify.sh
```

The CI also starts the release node and exercises the EVM JSON-RPC path end to end
(using `curl`):

```sh
sh scripts/evm-smoke.sh
```

This smoke test confirms that an RPC request deploys and calls the sample bytecode
and returns 42. It does **not** prove persistence across requests or ledger integration.

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
- `FLEUR_EVM_JOURNAL`: replayable EVM command journal path (default `./data/fleur-evm.jsonl`).

The server supports JSON-RPC 2.0 over `POST /rpc`. For example, query status:

```sh
curl -s http://127.0.0.1:8545/rpc \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"fleur_chainStatus"}'
```

For a disposable EVM sandbox smoke test, submit init bytecode that returns a tiny
runtime contract. The returned `returnData` should be a 32-byte hex value ending in
`2a` (decimal 42):

```sh
curl -s http://127.0.0.1:8545/rpc \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"fleur_evmPersistentDeployAndCall","params":{"initCode":"0x600a600c600039600a6000f3602a60005260206000f3"}}'
```

The persistent method appends the deployment/call command to the EVM JSONL journal and replays the command history on subsequent requests and restarts. The journal is still separate from chain blocks and ledger transactions; reported gas is diagnostic only and is not charged. The older `fleur_evmSandboxDeployAndCall` method remains available for isolated, stateless testing.

## Ten-day sprint target

1. Protocol rules and clean bootstrap.
2. Signed transaction envelope and key verification.
3. Block validation and parent continuity.
4. Append-only journal and replay validation.
5. Single-node RPC.
6. Integrate the REVM engine with an isolated deploy/call sandbox and add compatibility tests.
7. Connect EVM state, transaction validation, gas settlement, and rollback to chain blocks.
8. Multi-node synchronization and adversarial tests.
9. End-to-end testnet scripts and explorer-facing APIs.
10. CI evidence, operational docs, and limitations report.

The sprint target is a testnet prototype, not a security-audited mainnet.
