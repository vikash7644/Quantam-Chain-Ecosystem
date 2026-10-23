import TokenCard from "../components/TokenCard";

const mockTokens = [
  {
    symbol: "HAMMER",
    name: "fun.hammer",
    price: "$0.00042",
    marketCap: "$2.4M",
    volume: "$845K",
    trend: "Bullish",
    holders: "1,284",
    progress: 72,
  },
  {
    symbol: "SOLX",
    name: "SolanaX",
    price: "$0.00018",
    marketCap: "$1.1M",
    volume: "$420K",
    trend: "Hot",
    holders: "980",
    progress: 58,
  },
  {
    symbol: "PUMP",
    name: "PumpWave",
    price: "$0.00009",
    marketCap: "$680K",
    volume: "$310K",
    trend: "Rising",
    holders: "742",
    progress: 64,
  },
];

export default function HomePage() {
  return (
    <main style={{ background: '#0f0f13', color: 'white', minHeight: '100vh', padding: '32px 20px' }}>
      <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '32px' }}>
          <div>
            <div style={{ fontSize: '12px', letterSpacing: '0.2em', color: '#8b8b95', textTransform: 'uppercase' }}>
              fun.hammer
            </div>
            <h1 style={{ margin: '8px 0 0', fontSize: '36px' }}>Launch token on-chain</h1>
          </div>

          <button
            style={{
              background: '#7c3aed',
              color: 'white',
              border: 'none',
              borderRadius: '999px',
              padding: '12px 20px',
              fontWeight: 700,
              cursor: 'pointer',
            }}
          >
            Connect Wallet
          </button>
        </header>

        <section
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
            gap: '20px',
            marginTop: '24px',
          }}
        >
          {mockTokens.map((token) => (
            <TokenCard key={token.symbol} token={token} />
          ))}
        </section>
      </div>
    </main>
  );
}
