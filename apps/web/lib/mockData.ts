export type MockToken = {
  symbol: string;
  name: string;
  price: string;
  marketCap: string;
  volume: string;
  trend: string;
  holders: string;
  progress: number;
};

export const mockTokens: MockToken[] = [
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
  {
    symbol: "BOLT",
    name: "BoltRush",
    price: "$0.00012",
    marketCap: "$910K",
    volume: "$330K",
    trend: "Momentum",
    holders: "654",
    progress: 71,
  },
];
