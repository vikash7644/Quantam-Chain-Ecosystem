"use client";

import { FormEvent, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";

const DEFAULT_LAUNCH_FEE_SOL = 0.05;

export default function CreatePage() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();

  const [form, setForm] = useState({
    name: "",
    symbol: "",
    supply: "1000000",
    description: "",
    image: "",
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [txStatus, setTxStatus] = useState<string>("");

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    if (!connected || !publicKey) {
      setTxStatus("Connect your wallet first.");
      return;
    }

    setIsSubmitting(true);
    setTxStatus("Creating launch transaction...");

    try {
      const treasuryAddress = new PublicKey(
        process.env.NEXT_PUBLIC_LAUNCH_TREASURY ?? "11111111111111111111111111111111"
      );

      const amountLamports = Math.round(DEFAULT_LAUNCH_FEE_SOL * LAMPORTS_PER_SOL);

      const transaction = new Transaction();
      transaction.add(
        SystemProgram.transfer({
          fromPubkey: publicKey,
          toPubkey: treasuryAddress,
          lamports: amountLamports,
        })
      );

      transaction.feePayer = publicKey;
      const latestBlockhash = await connection.getLatestBlockhash();
      transaction.recentBlockhash = latestBlockhash.blockhash;

      const signature = await sendTransaction(transaction, connection);
      await connection.confirmTransaction({
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
        signature,
      });

      setTxStatus(`Launch fee paid successfully. Signature: ${signature}`);
      console.log("Token launch submitted:", form, { signature });
    } catch (error) {
      console.error("Launch transaction failed:", error);
      setTxStatus("Transaction failed. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <main className="page-shell narrow">
      <div className="card form-card">
        <p className="eyebrow">Launch</p>
        <h1>Create a new token</h1>

        <form onSubmit={handleSubmit} className="launch-form">
          <div className="grid-two">
            <label>
              <span>Token name</span>
              <input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="HammerCoin"
              />
            </label>

            <label>
              <span>Symbol</span>
              <input
                value={form.symbol}
                onChange={(e) => setForm({ ...form, symbol: e.target.value.toUpperCase() })}
                placeholder="HAMMER"
              />
            </label>
          </div>

          <div className="grid-two">
            <label>
              <span>Initial supply</span>
              <input
                type="number"
                value={form.supply}
                onChange={(e) => setForm({ ...form, supply: e.target.value })}
              />
            </label>

            <label>
              <span>Image URL</span>
              <input
                value={form.image}
                onChange={(e) => setForm({ ...form, image: e.target.value })}
                placeholder="https://..."
              />
            </label>
          </div>

          <label>
            <span>Description</span>
            <textarea
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
              placeholder="Describe your token, community angle, and launch story..."
            />
          </label>

          <button type="submit" className="primary-btn full-width" disabled={isSubmitting}>
            {isSubmitting ? "Submitting..." : "Launch token"}
          </button>

          {txStatus ? <div className="tx-status">{txStatus}</div> : null}
        </form>
      </div>
    </main>
  );
}
