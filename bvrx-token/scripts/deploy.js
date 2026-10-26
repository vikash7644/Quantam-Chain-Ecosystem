const { ethers } = require("hardhat");

async function main() {
  if (network.name !== "bscTestnet") {
    throw new Error("Safety guard: this script only permits bscTestnet deployment.");
  }

  const rawSupply = process.env.BVRX_INITIAL_SUPPLY;
  if (!rawSupply || !/^\d+$/.test(rawSupply) || BigInt(rawSupply) <= 0n) {
    throw new Error("Set BVRX_INITIAL_SUPPLY to a positive whole-token amount.");
  }

  const [deployer] = await ethers.getSigners();
  if (!deployer) {
    throw new Error("Missing deployer. Set BSC_TESTNET_PRIVATE_KEY in your local environment.");
  }

  console.log("Network:", network.name);
  console.log("Deployer:", deployer.address);
  console.log("Initial supply (whole tokens):", rawSupply);

  const Token = await ethers.getContractFactory("BVRXToken");
  const token = await Token.deploy(BigInt(rawSupply));
  await token.waitForDeployment();

  console.log("BVRX testnet contract:", await token.getAddress());
  console.log("Deployment transaction:", token.deploymentTransaction().hash);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
