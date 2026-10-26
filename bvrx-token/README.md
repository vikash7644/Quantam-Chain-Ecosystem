# BVRX Token (testnet-first)

This folder is an isolated Solidity project for a fixed-supply BVRX token intended for BNB Smart Chain. It is separate from the Fleur protocol implementation.

## Current status

- Contract source and automated tests are in the repository.
- GitHub Actions checks compile and test the contract.
- No token has been deployed by this commit.
- The sample supply in `.env.example` is a placeholder for local test deployment, not a finalized tokenomics decision.
- Mainnet deployment, DEX liquidity and any listing are explicitly out of scope until separately reviewed and approved.

## Token properties

- Name: `BVRX Token`
- Symbol: `BVRX`
- Decimals: 18
- Supply: set once at deployment as a positive whole-token amount
- Minting after deployment: no mint function
- Owner/admin privileges: none

## Run tests

Requires Node.js 20 or newer.

```sh
npm ci
npm test
npm run compile
```

## Testnet deployment

1. Create a local `.env` file based on `.env.example`.
2. Obtain test BNB from the official BNB Chain testnet faucet.
3. Set `BSC_TESTNET_PRIVATE_KEY` only in your local environment or an appropriately protected secret store. Never commit it or share it in chat.
4. Set `BVRX_INITIAL_SUPPLY` to the whole-token supply you have deliberately chosen.
5. Run `npm run deploy:testnet`.

The deployment script has a safety guard that refuses any network except `bscTestnet`. Review the contract, test results, token supply and deployment address before considering any mainnet launch.

Official references:
- BNB Chain docs: https://docs.bnbchain.org/
- BNB Smart Chain testnet explorer: https://testnet.bscscan.com/
- BNB Chain testnet faucet: https://docs.bnbchain.org/bnb-smart-chain/developers/faucet/
