"use client";

import { FormEvent, useState } from "react";

export default function CreatePage() {
  const [form, setForm] = useState({
    name: "",
    symbol: "",
    supply: "1000000",
    description: "",
    image: "",
  });

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    console.log("Token launch submitted:", form);
    alert("Token launch draft created successfully");
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

          <button type="submit" className="primary-btn full-width">Launch token</button>
        </form>
      </div>
    </main>
  );
}
