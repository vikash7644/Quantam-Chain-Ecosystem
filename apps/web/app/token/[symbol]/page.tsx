import BuySellPanel from "../../components/BuySellPanel";
import Header from "../../components/Header";
import { mockTokens } from "../../lib/mockData";

export default function TokenDetailPage({ params }: { params: { symbol: string } }) {
  const token = mockTokens.find((item) => item.symbol.toLowerCase() === params.symbol.toLowerCase()) ?? mockTokens[0];

  return (
    <main className="page-shell">
      <Header />

      <section className="token-detail">
        <div className="card token-summary">
          <div className="token-header-row">
            <div className="coin-badge">{token.symbol.slice(0, 2)}</div>
            <div>
              <p className="eyebrow">Token</p>
              <h1>{token.name}</h1>
              <p className="token-symbol">{token.symbol}</p>
            </div>
          </div>

          <div className="metrics-grid">
            <div>
              <span>Price</span>
              <strong>{token.price}</strong>
            </div>
            <div>
              <span>Market cap</span>
              <strong>{token.marketCap}</strong>
            </div>
            <div>
              <span>Volume</span>
              <strong>{token.volume}</strong>
            </div>
            <div>
              <span>Holders</span>
              <strong>{token.holders}</strong>
            </div>
          </div>

          <div className="chart-box">
            <div className="chart-bars">
              {[42, 58, 61, 64, 59, 72, 80, 76, 90, 96, 88, 110].map((height, index) => (
                <span key={index} style={{ height: `${height}%` }} />
              ))}
            </div>
          </div>
        </div>

        <BuySellPanel token={token} />
      </section>
    </main>
  );
}
