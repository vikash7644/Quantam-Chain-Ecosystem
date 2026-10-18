const test = require("node:test");
const assert = require("node:assert/strict");
const { ZenvoraChain, createWallet, signTransfer, FAUCET_AMOUNT } = require("../src/chain");

test("starts with a valid genesis block", () => {
  const chain = new ZenvoraChain();
  assert.equal(chain.info().height, 0);
  assert.equal(chain.verifyChain(), true);
});

test("faucet credits test tokens only after mining", () => {
  const chain = new ZenvoraChain();
  const wallet = createWallet();
  chain.faucet(wallet.address);
  assert.equal(chain.getBalance(wallet.address), FAUCET_AMOUNT);
  chain.mineBlock();
  assert.equal(chain.verifyChain(), true);
  assert.equal(chain.getBalance(wallet.address), FAUCET_AMOUNT);
});

test("signed transfer moves balances", () => {
  const chain = new ZenvoraChain();
  const alice = createWallet();
  const bob = createWallet();
  chain.faucet(alice.address);
  chain.mineBlock();
  chain.submitTransfer(signTransfer(alice, bob.address, 125));
  chain.mineBlock();
  assert.equal(chain.getBalance(alice.address), FAUCET_AMOUNT - 125);
  assert.equal(chain.getBalance(bob.address), 125);
  assert.equal(chain.verifyChain(), true);
});

test("rejects unsigned or tampered transfers", () => {
  const chain = new ZenvoraChain();
  const alice = createWallet();
  const bob = createWallet();
  chain.faucet(alice.address);
  chain.mineBlock();
  const tx = signTransfer(alice, bob.address, 100);
  tx.amount = 999;
  assert.throws(() => chain.submitTransfer(tx), /signature/i);
});

test("rejects faucet cap overflow", () => {
  const chain = new ZenvoraChain();
  const wallet = createWallet();
  for (let i = 0; i < 10; i++) chain.faucet(wallet.address);
  assert.throws(() => chain.faucet(wallet.address), /cap/i);
});
