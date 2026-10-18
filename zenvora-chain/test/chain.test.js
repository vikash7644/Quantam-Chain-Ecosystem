const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { ZenvoraChain, createWallet, signTransfer, FAUCET_AMOUNT } = require("../src/chain");

test("starts with a valid genesis block", () => {
  const chain = new ZenvoraChain();
  assert.equal(chain.info().height, 0);
  assert.equal(chain.verifyChain(), true);
});

test("faucet credits pending test tokens and confirms them after mining", () => {
  const chain = new ZenvoraChain();
  const wallet = createWallet();
  chain.faucet(wallet.address);
  assert.equal(chain.getConfirmedBalance(wallet.address), 0);
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

test("pending outgoing transfers reserve confirmed funds exactly once", () => {
  const chain = new ZenvoraChain();
  const alice = createWallet();
  const bob = createWallet();
  const carol = createWallet();
  const dave = createWallet();
  chain.faucet(alice.address);
  chain.mineBlock();

  chain.submitTransfer(signTransfer(alice, bob.address, 800));
  chain.submitTransfer(signTransfer(alice, carol.address, 200));
  assert.throws(
    () => chain.submitTransfer(signTransfer(alice, dave.address, 1)),
    /insufficient/i
  );
});

test("unconfirmed incoming transfers cannot be spent before mining", () => {
  const chain = new ZenvoraChain();
  const alice = createWallet();
  const bob = createWallet();
  const carol = createWallet();
  chain.faucet(alice.address);
  chain.mineBlock();
  chain.submitTransfer(signTransfer(alice, bob.address, 100));
  assert.equal(chain.getConfirmedBalance(bob.address), 0);
  assert.throws(
    () => chain.submitTransfer(signTransfer(bob, carol.address, 1)),
    /insufficient/i
  );
});

test("mined chain survives restart and validates saved data", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "zenvora-chain-"));
  const dataFile = path.join(directory, "chain.json");
  try {
    const chain = new ZenvoraChain({ dataFile });
    const wallet = createWallet();
    chain.faucet(wallet.address);
    chain.mineBlock();

    const restarted = new ZenvoraChain({ dataFile });
    assert.equal(restarted.info().height, 1);
    assert.equal(restarted.getConfirmedBalance(wallet.address), FAUCET_AMOUNT);
    assert.equal(restarted.verifyChain(), true);

    const saved = JSON.parse(fs.readFileSync(dataFile, "utf8"));
    saved.blocks[1].transactions[0].amount = 999999;
    fs.writeFileSync(dataFile, JSON.stringify(saved));
    assert.throws(() => new ZenvoraChain({ dataFile }), /verification/i);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
