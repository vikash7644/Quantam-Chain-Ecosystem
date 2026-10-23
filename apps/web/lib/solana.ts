import { Connection, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

export const connection = new Connection(
  process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "https://api.devnet.solana.com"
);

export async function getAddressBalance(address: string) {
  const publicKey = new PublicKey(address);
  const balanceInLamports = await connection.getBalance(publicKey);
  return balanceInLamports / LAMPORTS_PER_SOL;
}
