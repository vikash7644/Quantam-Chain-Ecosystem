import { createMint, mintTo, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { clusterApiUrl, Connection, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";

const rpcUrl = process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? clusterApiUrl("devnet");
const connection = new Connection(rpcUrl, "confirmed");

const privateKey = process.env.SOLANA_PRIVATE_KEY;
const amount = Number(process.env.MINT_INITIAL_SUPPLY ?? "1000000");
const decimals = Number(process.env.MINT_DECIMALS ?? "9");

if (!privateKey) {
  console.error("Missing SOLANA_PRIVATE_KEY in environment. Example: export SOLANA_PRIVATE_KEY='[1,2,3,...]'");
  process.exit(1);
}

const secretKey = Uint8Array.from(JSON.parse(privateKey));
const payer = Keypair.fromSecretKey(secretKey);

const createTokenMint = async () => {
  const mintAuthority = payer.publicKey;
  const freezeAuthority = payer.publicKey;

  const mint = await createMint(
    connection,
    payer,
    mintAuthority,
    freezeAuthority,
    decimals
  );

  const tokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    payer.publicKey
  );

  const amountRaw = BigInt(Math.floor(amount * 10 ** decimals));
  await mintTo(
    connection,
    payer,
    mint,
    tokenAccount.address,
    mintAuthority,
    amountRaw
  );

  console.log("✅ Token mint created successfully");
  console.log("Mint Address:", mint.toBase58());
  console.log("Owner Address:", payer.publicKey.toBase58());
  console.log("Token Account:", tokenAccount.address.toBase58());
  console.log("Initial Supply:", amount, "with", decimals, "decimals");
  console.log("RPC:", rpcUrl);
};

const airdrop = async () => {
  const balance = await connection.getBalance(payer.publicKey);
  if (balance < 0.1 * LAMPORTS_PER_SOL) {
    const airdropSig = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction({ signature: airdropSig, ...(await connection.getLatestBlockhash()) });
    console.log("💰 Airdrop requested for wallet:", payer.publicKey.toBase58());
  }
};

try {
  await airdrop();
  await createTokenMint();
} catch (error) {
  console.error("Token mint failed:", error);
  process.exit(1);
}
