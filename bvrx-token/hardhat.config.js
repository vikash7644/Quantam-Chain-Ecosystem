require("@nomicfoundation/hardhat-toolbox");
require("dotenv").config();

const accounts = process.env.BSC_TESTNET_PRIVATE_KEY
  ? [process.env.BSC_TESTNET_PRIVATE_KEY]
  : [];

module.exports = {
  solidity: {
    version: "0.8.24",
    settings: {
      optimizer: { enabled: true, runs: 200 }
    }
  },
  networks: {
    bscTestnet: {
      url: process.env.BSC_TESTNET_RPC_URL || "https://bsc-testnet-dataseed.bnbchain.org",
      chainId: 97,
      accounts
    }
  }
};
