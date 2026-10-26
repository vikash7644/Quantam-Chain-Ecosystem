# BVRX Token (testnet-first)

This folder is an isolated Solidity project for a fixed-supply BVRX token intended for BNB Smart Chain. It is separate from the Fleur Network protocol implementation.

![BVRX token emblem](assets/bvrx-logo.svg)

## Current status

- Contract source and automated tests are in the repository.
- GitHub Actions checks compile and test the contract.
- **Do not treat the token as deployed:** publish an address only after a successful testnet transaction is verified on BscScan.
- The intended initial supply for the planned testnet deployment is **1,000,000,000 BVRX**; deployment has not been confirmed by this README.
- Mainnet deployment, DEX liquidity, listings, staking, and governance are not claimed to be live.

## Token properties

- Name: `BVRX Token`
- Symbol: `BVRX`
- Standard: BEP-20-compatible ERC-20
- Decimals: 18
- Supply: selected once at deployment; current planned configuration is 1,000,000,000 whole tokens
- Minting after deployment: no mint function
- Owner/admin privileges: none

## Differentiation and utility plan

See [Utility and Tokenomics Roadmap](UTILITY_AND_TOKENOMICS.md). The goal is to prioritize verifiable ecosystem utility, transparent supply, public deployment evidence, and separately reviewed utility modules. Staking, rewards, governance and app integrations are **proposed future work**, not current contract features or guaranteed returns.

BVRX is a BEP-20 token project; **it is not the native coin of Fleur Network**. The token and protocol remain separate projects unless an integration is explicitly implemented and documented.

## Run tests

Requires Node.js 20 or newer.

```sh
npm ci
npm test
npm run compile
```

## Testnet deployment

1. Obtain test BNB from the official BNB Chain testnet faucet.
2. Store `BSC_TESTNET_PRIVATE_KEY` only in GitHub Actions Secrets or another protected secret store. Never commit it or share it in chat.
3. Set `BVRX_INITIAL_SUPPLY=1000000000` for the currently planned supply.
4. Run the GitHub Actions testnet deployment workflow or use the documented testnet deployment procedure.
5. Verify the transaction and contract address on the official BSC testnet explorer before publishing them.

The deployment script has a safety guard that refuses any network except `bscTestnet`. Review the contract, tests, supply and deployment address before considering any mainnet launch.

Official references:
- BNB Chain docs: https://docs.bnbchain.org/
- BNB Smart Chain testnet explorer: https://testnet.bscscan.com/
- BNB Chain testnet faucet: https://docs.bnbchain.org/bnb-smart-chain/developers/faucet/
