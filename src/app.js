const express = require('express');
const crypto = require('crypto');
const app = express();
app.use(express.json());

class Block {
    constructor(index, previousHash, timestamp, transactions) {
        this.index = index;
        this.previousHash = previousHash;
        this.timestamp = timestamp;
        this.transactions = transactions;
        this.hash = this.calculateHash();
    }
    calculateHash() {
        const blockString = this.index + this.previousHash + this.timestamp + JSON.stringify(this.transactions);
        return crypto.createHash('sha256').update(blockString).digest('hex');
    }
}

class QuantumChainEcosystem {
    constructor() {
        this.chain = [this.createGenesisBlock()];
        this.pendingTransactions = [];
        this.tokens = [];
        this.nfts = [];
        this.products = [];
    }
    createGenesisBlock() { return new Block(0, "0", Date.now(), { message: "Quantum Chain Genesis Block Live" }); }
    getLatestBlock() { return this.chain[this.chain.length - 1]; }
    createTransaction(fromAddress, toAddress, amount) {
        this.pendingTransactions.push({ fromAddress, toAddress, amount, timestamp: Date.now() });
    }
    minePendingTransactions() {
        const newBlock = new Block(this.getLatestBlock().index + 1, this.getLatestBlock().hash, Date.now(), this.pendingTransactions);
        this.chain.push(newBlock);
        this.pendingTransactions = [];
    }
}

const quantumNet = new QuantumChainEcosystem();
const MY_QUANTUM_WALLET = "0x3050cb3528Eb830abcb0d8f77bC7C35994452F05";
const RECEIVER_WALLET = "0x71C7656EC7ab88b098defB751B7401B5f6d8976F";

quantumNet.createTransaction(MY_QUANTUM_WALLET, RECEIVER_WALLET, "500 QTM");
quantumNet.createTransaction(RECEIVER_WALLET, MY_QUANTUM_WALLET, "150 QTM");
quantumNet.createTransaction("GENESIS", MY_QUANTUM_WALLET, "1000000 QTM");
quantumNet.minePendingTransactions();

console.log("=== QUANTAM CHAIN ECOSYSTEM LIVE ===");

app.get('/', (req, res) => res.send("QUANTUM CHAIN LIVE - /chain pe jao"));
app.get('/chain', (req, res) => res.json(quantumNet.chain));

app.post('/pump/create', (req, res) => {
    const { name, symbol } = req.body;
    const token = { name, symbol, creator: MY_QUANTUM_WALLET };
    quantumNet.tokens.push(token);
    res.json({ success: true, message: "Coin ban gaya!", token });
});

app.post('/dex/swap', (req, res) => {
    const { fromToken, toToken, amount } = req.body;
    res.json({ success: true, message: `${amount} ${fromToken} -> ${toToken} swap done` });
});

app.post('/nft/mint', (req, res) => {
    const nft = { id: quantumNet.nfts.length + 1, name: req.body.name, owner: MY_QUANTUM_WALLET };
    quantumNet.nfts.push(nft);
    res.json({ success: true, nft });
});

app.post('/ecommerce/add', (req, res) => {
    quantumNet.products.push(req.body);
    res.json({ success: true, message: "Product added" });
});

// RENDER FIX - Yahi change hai
const PORT = process.env.PORT || 3000;
app.listen(PORT, () => console.log(`QUANTAM CHAIN LIVE on ${PORT}`));
