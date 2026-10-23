# Fleur Wallet — Telegram Mini App starter

Fleur Wallet is a **prototype**, built as a separate folder inside Quantam-Chain-Ecosystem. It does not change Zenvora Lang or the repository's main branch.

## Included in this starter
- Telegram Mini App SDK bootstrap (also opens in a normal browser).
- Client-side EVM wallet creation using ethers.js; key material is generated in the browser.
- Password-encrypted keystore export/import flow. Passwords are not stored.
- Demo reward points stored locally for UI testing; these are **not tokens and cannot be withdrawn or traded**.
- Token / swap interface marked as not live until a chain, audited contracts, RPC and liquidity are configured.

## Run locally
Serve this folder using any static web server and open `index.html`. For Telegram, configure a Telegram bot with a Mini App URL using HTTPS.

## Security and production blockers
This is not production-ready and must not hold real funds yet.
- Audit wallet generation, keystore handling, dependencies, CSP and supply-chain risks.
- Use a reviewed wallet library and carefully reviewed transaction-signing UX.
- Never send private keys, seed phrases or keystore passwords to a backend or Telegram bot.
- Do not store plaintext private keys or seed phrases in localStorage.
- Add backend-authenticated reward accounting with anti-abuse controls before describing rewards as withdrawable.
- Configure and test the real Fleur Network chain ID, RPC, native currency, token contracts and explorer only after the network exists.
- Add audited swap routing/liquidity and any required payment/compliance integrations before enabling real trades.
- Security review and testnet tests are mandatory before mainnet use.

## Current status
UI and wallet creation are prototype functionality. Mining is demo points only. Live chain transactions, real mining, buy/sell/swap, token issuance and revenue are **not implemented**.
