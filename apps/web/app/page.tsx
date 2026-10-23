import Header from "../components/Header";
import TokenCard from "../components/TokenCard";
import { mockTokens } from "../lib/mockData";

export default function HomePage() {
  return (
    <main className="page-shell">
      <Header />

      <section className="hero">
        <div>
          <p className="eyebrow">fun.hammer</p>
          <h1>Launch the next viral coin before the market wakes up.</h1>
          <p className="hero-copy">
            Create, trade, and track pump-style tokens in a fast, on-chain launchpad tuned for speed.
          </p>

          <div className="hero-actions">
            <a href="/create" className="primary-btn">Create token</a>
            <a href="#market" className="secondary-btn">Explore market</a>
          </div>
        </div>

        <div className="hero-panel">
          <div className="stat-box">
            <span>24H Volume</span>
            <strong>$42.8M</strong>
          </div>
          <div className="stat-box">
            <span>New launches</span>
            <strong>128</strong>
          </div>
          <div className="stat-box">
            <span>Top trend</span>
            <strong>HAMMER</strong>
          </div>
        </div>
      </section>

      <section id="market" className="market-section">
        <div className="section-header">
          <div>
            <p className="eyebrow">Market</p>
            <h2>Trending launches</h2>
          </div>
          <a href="/create" className="text-link">Launch now</a>
        </div>

        <div className="token-grid">
          {mockTokens.map((token) => (
            <TokenCard key={token.symbol} token={token} />
          ))}
        </div>
      </section>
    </main>
  );
}
