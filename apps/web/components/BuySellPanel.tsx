"use client";

import { useState } from "react";

export default function BuySellPanel({ token }: { token: { symbol: string; price: string } }) {
  const [mode, setMode] = useState<"buy" | "sell">("buy");
  const [amount, setAmount] = useState("0.5");

  return (
    <div className="card trade-panel">
      <div className="trade-tabs">
        <button className={mode === "buy" ? "active" : ""} onClick={() => setMode("buy")}>
          Buy
        </button>
        <button className={mode === "sell" ? "active" : ""} onClick={() => setMode("sell")}>
          Sell
        </button>
      </div>

      <div className="trade-box">
        <label>
          <span>{mode === "buy" ? "Pay with SOL" : "Send token"}</span>
          <input value={amount} onChange={(e) => setAmount(e.target.value)} />
        </label>

        <div className="trade-summary">
          <span>You will receive</span>
          <strong>
            {mode === "buy" ? `${amount} ${token.symbol}` : `${amount} SOL`}
          </strong>
        </div>

        <button className="primary-btn full-width">
          {mode === "buy" ? `Buy ${token.symbol}` : `Sell ${token.symbol}`}
        </button>
      </div>
    </div>
  );
}
