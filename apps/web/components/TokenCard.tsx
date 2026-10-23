type TokenCardProps = {
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
    <div
      style={{
        background: 'linear-gradient(180deg, #171c2a 0%, #0f1320 100%)',
        border: '1px solid rgba(255,255,255,0.08)',
        borderRadius: '24px',
        padding: '20px',
        boxShadow: '0 12px 30px rgba(0,0,0,0.25)',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <div style={{ fontSize: '12px', color: '#8b8b95' }}>Token</div>
          <h2 style={{ margin: '4px 0', fontSize: '24px' }}>{token.symbol}</h2>
        </div>
        <div
          style={{
            background: '#1a2336',
            borderRadius: '999px',
            padding: '6px 10px',
            fontSize: '12px',
            color: '#7dd3fc',
            border: '1px solid rgba(125, 211, 252, 0.2)',
          }}
        >
          {token.trend}
        </div>
      </div>

      <div style={{ marginTop: '14px', fontSize: '18px', fontWeight: 700 }}>{token.name}</div>

      <div style={{ marginTop: '18px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
        <div>
          <div style={{ color: '#8b8b95', fontSize: '12px' }}>Price</div>
          <div style={{ fontWeight: 700 }}>{token.price}</div>
        </div>
        <div>
          <div style={{ color: '#8b8b95', fontSize: '12px' }}>Market cap</div>
          <div style={{ fontWeight: 700 }}>{token.marketCap}</div>
        </div>
      </div>

      <div style={{ marginTop: '18px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
        <div>
          <div style={{ color: '#8b8b95', fontSize: '12px' }}>Volume</div>
          <div style={{ fontWeight: 700 }}>{token.volume}</div>
        </div>
        <div>
          <div style={{ color: '#8b8b95', fontSize: '12px' }}>Holders</div>
          <div style={{ fontWeight: 700 }}>{token.holders}</div>
        </div>
      </div>

      <div style={{ marginTop: '22px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', color: '#8b8b95', fontSize: '12px', marginBottom: '8px' }}>
          <span>Launch momentum</span>
          <span>{token.progress}%</span>
        </div>
        <div style={{ height: '8px', borderRadius: '999px', background: '#1d2332', overflow: 'hidden' }}>
          <div
            style={{
              width: `${token.progress}%`,
              height: '100%',
              background: 'linear-gradient(90deg, #7c3aed, #22c55e)',
              borderRadius: '999px',
            }}
          />
        </div>
      </div>

      <div style={{ marginTop: '22px', display: 'flex', gap: '12px' }}>
        <button
          style={{
            flex: 1,
            background: '#22c55e',
            color: 'black',
            border: 'none',
            borderRadius: '12px',
            padding: '12px 0',
            fontWeight: 800,
            cursor: 'pointer',
          }}
        >
          Buy
        </button>
        <button
          style={{
            flex: 1,
            background: '#ef4444',
            color: 'white',
            border: 'none',
            borderRadius: '12px',
            padding: '12px 0',
            fontWeight: 800,
            cursor: 'pointer',
          }}
        >
          Sell
        </button>
      </div>
    </div>
  );
}
