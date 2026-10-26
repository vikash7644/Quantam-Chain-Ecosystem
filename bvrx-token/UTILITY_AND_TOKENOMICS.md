# BVRX: utility-first roadmap

BVRX is being developed as a fixed-supply BEP-20 token for the Quantam Chain Ecosystem. A token contract by itself does **not** provide staking, governance, discounts, rewards, or a DEX. Those need separate contracts, product integrations, testing, and security review.

## What can make BVRX distinct

1. **Transparent fixed supply** — the current token contract has no post-deployment mint function and no owner/admin controls.
2. **Utility before hype** — publish real product integrations and measurable usage before promoting token demand.
3. **Public verification** — publish the verified contract source, deployment transaction, supply, and official explorer link after a successful testnet deployment.
4. **Separate, reviewable modules** — keep token transfers simple; implement optional utility in separate contracts so each module can be tested and reviewed independently.
5. **Open development** — track each utility as an issue/PR with tests, threat model, and deployment notes.

## Proposed phases (not yet implemented)

| Phase | Feature | Requirements before release |
|---|---|---|
| 1 | Testnet token + explorer verification | Successful deployment, transaction/address recorded, reproducible CI |
| 2 | Ecosystem utility page + token dashboard | Accurate live contract data; no fabricated metrics |
| 3 | Governance signaling | Clear voting rules; no token-holder admin powers added to the token |
| 4 | Optional staking/rewards module | Separate audited/reviewed contract, disclosed reward source and risks |
| 5 | App/game/marketplace integrations | Working integrations and abuse controls demonstrated in tests |

## Important boundaries

- **No yield promise:** staking or rewards are ideas, not guaranteed returns.
- **No false audit claims:** do not label the project audited until an independent review is completed and published.
- **No false deployment claims:** no contract address should be published until a successful transaction is independently verified.
- **BVRX is not the native coin of Fleur Network.** The BVRX BEP-20 token project and the Fleur Network protocol are separate projects unless a future, explicitly documented integration is implemented.
- **No mainnet launch or liquidity claims** are made by this roadmap.

## Current contract baseline

- Name: BVRX Token
- Symbol: BVRX
- Standard: BEP-20-compatible ERC-20
- Decimals: 18
- Initial supply: chosen at deployment; current intended configuration is 1,000,000,000 whole tokens
- Post-deployment minting: not supported by the current contract
- Contract owner/admin: none

The intended supply configuration is not proof of a deployment. Update this page with the testnet address and transaction only after verifying them on the official BSC testnet explorer.
