const http = require("node:http");
const { URL } = require("node:url");
const path = require("node:path");
const { ZenvoraChain, createWallet, signTransfer } = require("./chain");

const dataFile = process.env.ZENVORA_DATA_FILE || path.join(__dirname, "..", "data", "chain.json");
const chain = new ZenvoraChain({ dataFile });
const PORT = Number(process.env.PORT || 8545);
const HOST = process.env.HOST || "127.0.0.1";

function send(res, status, body) {
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
    "access-control-allow-origin": "http://localhost:3000",
    "access-control-allow-methods": "GET,POST,OPTIONS",
    "access-control-allow-headers": "content-type"
  });
  res.end(JSON.stringify(body, null, 2));
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    let raw = "";
    req.on("data", chunk => {
      raw += chunk;
      if (raw.length > 64 * 1024) {
        reject(new Error("Request body too large."));
        req.destroy();
      }
    });
    req.on("end", () => {
      try { resolve(raw ? JSON.parse(raw) : {}); }
      catch { reject(new Error("Invalid JSON body.")); }
    });
    req.on("error", reject);
  });
}

const server = http.createServer(async (req, res) => {
  if (req.method === "OPTIONS") return send(res, 204, {});
  const url = new URL(req.url, "http://localhost");
  try {
    if (req.method === "GET" && url.pathname === "/") return send(res, 200, chain.info());
    if (req.method === "GET" && url.pathname === "/blocks") return send(res, 200, chain.blocks);
    if (req.method === "GET" && url.pathname.startsWith("/blocks/")) {
      const block = chain.getBlock(Number(url.pathname.slice("/blocks/".length)));
      return block ? send(res, 200, block) : send(res, 404, { error: "Block not found." });
    }
    if (req.method === "GET" && url.pathname === "/health") return send(res, 200, { ok: true });
    if (req.method === "GET" && url.pathname === "/balance") {
      const address = url.searchParams.get("address");
      if (!address) return send(res, 400, { error: "address query parameter required." });
      const confirmedBalance = chain.getConfirmedBalance(address);
      const pendingPreviewBalance = chain.getBalance(address);
      return send(res, 200, {
        address,
        confirmedBalance,
        pendingDelta: pendingPreviewBalance - confirmedBalance,
        pendingPreviewBalance,
        symbol: "tZVR",
        note: "Only confirmedBalance is spendable; pendingPreviewBalance includes unconfirmed transactions."
      });
    }
    if (req.method === "POST" && url.pathname === "/wallet/create") {
      const wallet = createWallet();
      return send(res, 201, {
        address: wallet.address,
        publicKey: wallet.publicKey,
        privateKey: wallet.privateKey,
        warning: "Local test wallet only. Never reuse this key or send real funds to it."
      });
    }
    if (req.method === "POST" && url.pathname === "/faucet") {
      const body = await readJson(req);
      const tx = chain.faucet(body.address);
      return send(res, 202, { accepted: true, transaction: tx, note: "Mine a block to confirm this test-faucet transaction." });
    }
    if (req.method === "POST" && url.pathname === "/transactions") {
      const body = await readJson(req);
      return send(res, 202, chain.submitTransfer(body));
    }
    if (req.method === "POST" && url.pathname === "/mine") {
      return send(res, 201, chain.mineBlock());
    }
    if (req.method === "GET" && url.pathname === "/verify") {
      return send(res, 200, { valid: chain.verifyChain() });
    }
    return send(res, 404, { error: "Route not found." });
  } catch (error) {
    return send(res, 400, { error: error.message || "Request failed." });
  }
});

server.listen(PORT, HOST, () => {
  console.log("Zenvora Chain Devnet listening on http://" + HOST + ":" + PORT);
  console.log("Local testnet only — tZVR has no monetary value.");
});
