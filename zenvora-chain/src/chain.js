const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const CHAIN_ID = "ZENVORA-DEVNET-1";
const SYMBOL = "tZVR";
const FAUCET_AMOUNT = 1000;
const FAUCET_CAP = 10000;

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function addressFromPublicKey(publicKey) {
  return "zvr_" + sha256(publicKey).slice(0, 40);
}

function transactionPayload(tx) {
  return JSON.stringify({
    chainId: tx.chainId,
    type: tx.type,
    from: tx.from,
    to: tx.to,
    amount: tx.amount,
    publicKey: tx.publicKey
  });
}

function verifyTransferSignature(tx) {
  try {
    if (!tx.publicKey || !tx.signature) return false;
    if (addressFromPublicKey(tx.publicKey) !== tx.from) return false;
    return crypto.verify(
      null,
      Buffer.from(transactionPayload(tx)),
      tx.publicKey,
      Buffer.from(tx.signature, "base64")
    );
  } catch {
    return false;
  }
}

function hashBlock(block) {
  const { hash, ...body } = block;
  return sha256(JSON.stringify(body));
}

class ZenvoraChain {
  constructor({ dataFile = null } = {}) {
    this.dataFile = dataFile;
    this.pending = [];
    this.blocks = [{
      index: 0,
      timestamp: new Date(0).toISOString(),
      chainId: CHAIN_ID,
      transactions: [],
      previousHash: "0".repeat(64),
      validator: "zenvora-devnet",
      hash: ""
    }];
    this.blocks[0].hash = hashBlock(this.blocks[0]);

    if (this.dataFile && fs.existsSync(this.dataFile)) {
      const saved = JSON.parse(fs.readFileSync(this.dataFile, "utf8"));
      if (!saved || saved.chainId !== CHAIN_ID || !Array.isArray(saved.blocks) || saved.blocks.length === 0) {
        throw new Error("Saved chain data has an invalid format; refusing to start.");
      }
      this.blocks = saved.blocks;
      if (!this.verifyChain()) {
        throw new Error("Saved chain data failed verification; refusing to start.");
      }
    }
  }

  persist() {
    if (!this.dataFile) return;
    const directory = path.dirname(this.dataFile);
    fs.mkdirSync(directory, { recursive: true });
    const temporary = this.dataFile + ".tmp";
    fs.writeFileSync(temporary, JSON.stringify({
      chainId: CHAIN_ID,
      blocks: this.blocks
    }, null, 2), { mode: 0o600 });
    fs.renameSync(temporary, this.dataFile);
  }

  info() {
    return {
      name: "Zenvora Chain Devnet",
      chainId: CHAIN_ID,
      nativeSymbol: SYMBOL,
      network: "LOCAL_TESTNET_ONLY",
      height: this.blocks.length - 1,
      pendingTransactions: this.pending.length,
      valid: this.verifyChain(),
      warning: "Test coins have no monetary value. This prototype is not production-secure."
    };
  }

  getConfirmedBalance(address) {
    let balance = 0;
    for (const block of this.blocks) {
      for (const tx of block.transactions) {
        if (tx.to === address) balance += tx.amount;
        if (tx.type === "transfer" && tx.from === address) balance -= tx.amount;
      }
    }
    return balance;
  }

  // Includes pending transactions; this is a preview, not a confirmed balance.
  getBalance(address) {
    let balance = this.getConfirmedBalance(address);
    for (const tx of this.pending) {
      if (tx.to === address) balance += tx.amount;
      if (tx.type === "transfer" && tx.from === address) balance -= tx.amount;
    }
    return balance;
  }

  faucet(address) {
    if (typeof address !== "string" || !/^zvr_[a-f0-9]{40}$/.test(address)) {
      throw new Error("A valid Zenvora test address is required.");
    }
    const alreadyReceived = this.blocks.flatMap(b => b.transactions)
      .filter(tx => tx.type === "faucet" && tx.to === address)
      .reduce((sum, tx) => sum + tx.amount, 0)
      + this.pending.filter(tx => tx.type === "faucet" && tx.to === address)
        .reduce((sum, tx) => sum + tx.amount, 0);
    if (alreadyReceived + FAUCET_AMOUNT > FAUCET_CAP) {
      throw new Error("Test faucet cap reached for this address.");
    }
    const tx = {
      chainId: CHAIN_ID, type: "faucet", from: "TEST_FAUCET",
      to: address, amount: FAUCET_AMOUNT, publicKey: "", signature: ""
    };
    this.pending.push(tx);
    return tx;
  }

  submitTransfer(tx) {
    if (!tx || tx.chainId !== CHAIN_ID || tx.type !== "transfer") {
      throw new Error("Invalid transaction or wrong chainId.");
    }
    if (typeof tx.to !== "string" || !/^zvr_[a-f0-9]{40}$/.test(tx.to)) {
      throw new Error("Invalid recipient address.");
    }
    if (!Number.isSafeInteger(tx.amount) || tx.amount <= 0) {
      throw new Error("Amount must be a positive safe integer.");
    }
    if (!verifyTransferSignature(tx)) throw new Error("Invalid transaction signature.");
    const reserved = this.pending
      .filter(item => item.type === "transfer" && item.from === tx.from)
      .reduce((sum, item) => sum + item.amount, 0);
    // Only confirmed funds are spendable; reserve prior pending outgoing transfers once.
    if (this.getConfirmedBalance(tx.from) - reserved < tx.amount) {
      throw new Error("Insufficient test balance.");
    }
    this.pending.push({
      chainId: CHAIN_ID, type: "transfer", from: tx.from,
      to: tx.to, amount: tx.amount, publicKey: tx.publicKey,
      signature: tx.signature
    });
    return { accepted: true, pending: this.pending.length };
  }

  mineBlock() {
    const previous = this.blocks[this.blocks.length - 1];
    const block = {
      index: previous.index + 1,
      timestamp: new Date().toISOString(),
      chainId: CHAIN_ID,
      transactions: this.pending.splice(0),
      previousHash: previous.hash,
      validator: "zenvora-devnet",
      hash: ""
    };
    block.hash = hashBlock(block);
    this.blocks.push(block);
    if (!this.verifyChain()) {
      this.blocks.pop();
      this.pending.unshift(...block.transactions);
      throw new Error("Block rejected: chain verification failed.");
    }
    try {
      this.persist();
    } catch (error) {
      this.blocks.pop();
      this.pending.unshift(...block.transactions);
      throw new Error("Block was not committed because chain persistence failed: " + error.message);
    }
    return block;
  }

  verifyChain() {
    const balances = new Map();
    const faucetTotals = new Map();
    for (let i = 0; i < this.blocks.length; i++) {
      const block = this.blocks[i];
      if (block.index !== i || block.chainId !== CHAIN_ID) return false;
      if (block.hash !== hashBlock(block)) return false;
      if (i === 0) {
        if (block.previousHash !== "0".repeat(64)) return false;
      } else if (block.previousHash !== this.blocks[i - 1].hash) return false;

      for (const tx of block.transactions) {
        if (tx.chainId !== CHAIN_ID || !Number.isSafeInteger(tx.amount) || tx.amount <= 0) return false;
        if (tx.type === "faucet") {
          if (tx.from !== "TEST_FAUCET") return false;
          const total = (faucetTotals.get(tx.to) || 0) + tx.amount;
          if (total > FAUCET_CAP) return false;
          faucetTotals.set(tx.to, total);
          balances.set(tx.to, (balances.get(tx.to) || 0) + tx.amount);
        } else if (tx.type === "transfer") {
          if (!verifyTransferSignature(tx)) return false;
          const fromBalance = balances.get(tx.from) || 0;
          if (fromBalance < tx.amount) return false;
          balances.set(tx.from, fromBalance - tx.amount);
          balances.set(tx.to, (balances.get(tx.to) || 0) + tx.amount);
        } else return false;
      }
    }
    return true;
  }

  getBlock(index) {
    if (!Number.isInteger(index) || index < 0) return null;
    return this.blocks[index] || null;
  }
}

function createWallet() {
  const pair = crypto.generateKeyPairSync("ed25519");
  const publicKey = pair.publicKey.export({ type: "spki", format: "pem" });
  const privateKey = pair.privateKey.export({ type: "pkcs8", format: "pem" });
  return { address: addressFromPublicKey(publicKey), publicKey, privateKey };
}

function signTransfer(wallet, to, amount) {
  const tx = {
    chainId: CHAIN_ID, type: "transfer", from: wallet.address,
    to, amount, publicKey: wallet.publicKey
  };
  const privateKey = crypto.createPrivateKey(wallet.privateKey);
  tx.signature = crypto.sign(null, Buffer.from(transactionPayload(tx)), privateKey).toString("base64");
  return tx;
}

module.exports = {
  ZenvoraChain, createWallet, signTransfer, CHAIN_ID, SYMBOL,
  FAUCET_AMOUNT, addressFromPublicKey
};
