const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("BVRXToken", function () {
  const wholeTokens = 1_000_000_000n;

  async function deployToken(supply = wholeTokens) {
    const [deployer, recipient] = await ethers.getSigners();
    const Token = await ethers.getContractFactory("BVRXToken");
    const token = await Token.deploy(supply);
    await token.waitForDeployment();
    return { token, deployer, recipient, supply };
  }

  it("sets the expected name, symbol and decimals", async function () {
    const { token } = await deployToken();
    expect(await token.name()).to.equal("BVRX Token");
    expect(await token.symbol()).to.equal("BVRX");
    expect(await token.decimals()).to.equal(18);
  });

  it("mints the configured initial supply to the deployer exactly once", async function () {
    const { token, deployer, supply } = await deployToken();
    const expected = supply * 10n ** 18n;
    expect(await token.totalSupply()).to.equal(expected);
    expect(await token.balanceOf(deployer.address)).to.equal(expected);
  });

  it("transfers tokens and updates balances", async function () {
    const { token, deployer, recipient, supply } = await deployToken();
    const amount = 25n * 10n ** 18n;
    await expect(token.transfer(recipient.address, amount))
      .to.emit(token, "Transfer")
      .withArgs(deployer.address, recipient.address, amount);
    expect(await token.balanceOf(recipient.address)).to.equal(amount);
    expect(await token.totalSupply()).to.equal(supply * 10n ** 18n);
  });

  it("rejects a zero initial supply", async function () {
    const Token = await ethers.getContractFactory("BVRXToken");
    await expect(Token.deploy(0)).to.be.revertedWith("BVRX: supply must be positive");
  });

  it("does not expose a mint function", async function () {
    const { token } = await deployToken();
    expect(token.interface.hasFunction("mint(address,uint256)")).to.equal(false);
  });
});
