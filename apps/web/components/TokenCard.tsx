import TokenCard from "../components/TokenCard";

export type TokenCardProps = {
  token: {
    symbol: string;
    name: string;
    price: string;
    marketCap: string;
    volume: string;
    trend: string;
    holders: string;
    progress: number;
  };
};

export default function TokenCard({ token }: TokenCardProps) {
  return (
    <a href={`/token/${token.symbol.toLowerCase()}`} className="token-card">
      <div className="token-card-head">
        <div className="token-meta">
          <div className="coin-badge small">{token.symbol.slice(0, 2)}</div>
          <div>
            <h3>{token.symbol}</h3>
            <p>{token.name}</p>
          </div>
        </div>
        <span className="pill">{token.trend}</span>
      </div>

      <div className="token-price">{token.price}</div>

      <div className="token-stats">
        <div>
          <span>MC</span>
          <strong>{token.marketCap}</strong>
        </div>
        <div>
          <span>Vol</span>
          <strong>{token.volume}</strong>
        </div>
        <div>
          <span>Holders</span>
          <strong>{token.holders}</strong>
        </div>
      </div>

      <div className="progress-wrap">
        <div className="progress-labels">
          <span>Momentum</span>
          <strong>{token.progress}%</strong>
        </div>
        <div className="progress-bar">
          <span style={{ width: `${token.progress}%` }} />
        </div>
      </div>
    </a>
  );
}
