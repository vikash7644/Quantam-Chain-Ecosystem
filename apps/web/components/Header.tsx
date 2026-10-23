"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useEffect, useState } from "react";
import { getAddressBalance } from "../lib/solana";

export default function Header() {
  const { publicKey, connected } = useWallet();
  const [balance, setBalance] = useState<number | null>(null);

  useEffect(() => {
    const loadBalance = async () => {
      if (!publicKey) {
        setBalance(null);
        return;
      }

      try {
        const walletBalance = await getAddressBalance(publicKey.toBase58());
        setBalance(walletBalance);
      } catch {
        setBalance(null);
      }
    };

    loadBalance();
  }, [publicKey, connected]);

  return (
    <header className="topbar">
      <a href="/" className="brand">
        <span className="brand-mark">f</span>
        fun.hammer
      </a>

      <nav className="nav">
        <a href="#market">Market</a>
        <a href="/create">Launch</a>
        <a href="#">Discover</a>
      </nav>

      <div className="wallet-stack">
        {connected && publicKey && balance !== null ? (
          <span className="wallet-balance">{balance.toFixed(3)} SOL</span>
        ) : null}
        <WalletMultiButton className="wallet-btn" />
      </div>
    </header>
  );
}
