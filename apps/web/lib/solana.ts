import { Connection, PublicKey } from '@solana/web3.js';

export const connection = new Connection(
  process.env.NEXT_PUBLIC_RPC_URL || 'https://api.devnet.solana.com'
);

export async function getBalance(address: string) {
  const publicKey = new PublicKey(address);
  const balance = await connection.getBalance(publicKey);
  return balance / 1_000_000_000;
}
