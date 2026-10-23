export default function Header() {
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

      <button className="wallet-btn">Connect wallet</button>
    </header>
  );
}
