# Zenvora Chain Devnet — MVP

A small, local-only blockchain prototype for learning and testing the Zenvora ecosystem. The native test asset is **tZVR**. It is not the real ZVR asset and has no monetary value.

## Current scope

- Genesis block and SHA-256 block hashes
- Local block production and chain-link validation
- Ed25519 wallet keys and signed test transfers
- Test faucet with a per-address cap
- Balance, block, health, and verification endpoints
- Automated tests for genesis, faucet, transfers, tampered signatures, and faucet caps

## Requirements

Node.js 20 or newer. No third-party packages are needed. Mined blocks are saved to a local JSON file by default.

## Run locally

```sh
cd zenvora-chain
npm test
npm start
```

The server binds to `127.0.0.1:8545` by default, so it is not exposed to the public internet. Mined blocks are saved to `zenvora-chain/data/chain.json` by default; set `ZENVORA_DATA_FILE` to choose another file path. Pending transactions are intentionally discarded on restart. To change the port, set `PORT`. Do not set `HOST=0.0.0.0` on an untrusted network.

## Quick API test

Create a test wallet:

```sh
curl -X POST http://127.0.0.1:8545/wallet/create
```

Request faucet tokens (replace the address with the wallet address returned above):

```sh
curl -X POST http://127.0.0.1:8545/faucet \
  -H 'content-type: application/json' \
  -d '{"address":"zvr_REPLACE_WITH_40_HEX_CHARACTERS"}'
```

Mine pending transactions:

```sh
curl -X POST http://127.0.0.1:8545/mine
```

Inspect the chain and a balance (the response separates confirmed funds from pending preview):

```sh
curl http://127.0.0.1:8545/
curl http://127.0.0.1:8545/blocks
curl 'http://127.0.0.1:8545/balance?address=zvr_REPLACE_WITH_ADDRESS'
curl http://127.0.0.1:8545/verify
```

## Important limitations

This is a **single-process devnet prototype**, not a public multi-node testnet or production blockchain. Mined blocks are persisted locally as JSON and checked on startup, but this is not a production-grade database or backup strategy. Pending transactions are in memory and reset when the process restarts. There is no peer-to-peer consensus, transaction fees, contract VM, or production wallet protection. Faucet and mining endpoints are intended for local development only. Do not use real funds or reuse these keys. A real public testnet needs persistent storage, peer networking, consensus design, rate limits, independent security review, and monitored deployments.
