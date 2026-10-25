#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CRATE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$CRATE_DIR"

PORT=${FLEUR_SMOKE_PORT:-18545}
TMP_DIR=$(mktemp -d)
PID=""
cleanup() {
  if [ -n "$PID" ]; then
    kill "$PID" 2>/dev/null || true
    wait "$PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

FLEUR_RPC_ADDR="127.0.0.1:$PORT" \
FLEUR_JOURNAL="$TMP_DIR/fleur-chain.jsonl" \
FLEUR_EVM_JOURNAL="$TMP_DIR/fleur-evm.jsonl" \
  cargo run --release --bin node >"$TMP_DIR/node.log" 2>&1 &
PID=$!

READY=0
i=0
while [ "$i" -lt 40 ]; do
  if curl -fsS "http://127.0.0.1:$PORT/rpc" \
    -H 'content-type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"fleur_chainStatus"}' \
    >"$TMP_DIR/status.json" 2>/dev/null; then
    READY=1
    break
  fi
  if ! kill -0 "$PID" 2>/dev/null; then
    cat "$TMP_DIR/node.log"
    echo "Fleur RPC node exited before becoming ready." >&2
    exit 1
  fi
  sleep 1
  i=$((i + 1))
done

if [ "$READY" -ne 1 ]; then
  cat "$TMP_DIR/node.log"
  echo "Timed out waiting for Fleur RPC node." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":10,"method":"eth_chainId"}' \
  >"$TMP_DIR/chain-id.json"
if ! grep -q '"result":"0xf168"' "$TMP_DIR/chain-id.json"; then
  cat "$TMP_DIR/chain-id.json"
  echo "Ethereum RPC smoke test failed: unexpected development chain ID." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":11,"method":"eth_blockNumber"}' \
  >"$TMP_DIR/block-number.json"
if ! grep -q '"result":"0x0"' "$TMP_DIR/block-number.json"; then
  cat "$TMP_DIR/block-number.json"
  echo "Ethereum RPC smoke test failed: expected genesis block height 0." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"fleur_evmPersistentDeployAndCall","params":{"initCode":"0x600a600c600039600a6000f3602a60005260206000f3"}}' \
  >"$TMP_DIR/evm.json"

if ! grep -q '"returnData":"000000000000000000000000000000000000000000000000000000000000002a"' "$TMP_DIR/evm.json"; then
  cat "$TMP_DIR/evm.json"
  echo "EVM smoke test failed: expected return value 42." >&2
  exit 1
fi

ADDRESS=$(sed -n 's/.*"contractAddress":"\([^"]*\)".*/\1/p' "$TMP_DIR/evm.json")
if [ -z "$ADDRESS" ]; then
  cat "$TMP_DIR/evm.json"
  echo "EVM smoke test failed: deployment response did not include contract address." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":12,\"method\":\"fleur_evmPersistentCall\",\"params\":{\"contractAddress\":\"$ADDRESS\"}}" \
  >"$TMP_DIR/evm-existing-call.json"

if ! grep -q '"returnData":"000000000000000000000000000000000000000000000000000000000000002a"' "$TMP_DIR/evm-existing-call.json"; then
  cat "$TMP_DIR/evm-existing-call.json"
  echo "EVM existing-contract call failed: expected return value 42." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":3,"method":"fleur_evmPersistentDeployAndCall","params":{"initCode":"0x600a600c600039600a6000f3602a60005260206000f3"}}' \
  >"$TMP_DIR/evm-second.json"

if ! grep -q '"returnData":"000000000000000000000000000000000000000000000000000000000000002a"' "$TMP_DIR/evm-second.json"; then
  cat "$TMP_DIR/evm-second.json"
  echo "Second EVM journal replay failed: expected return value 42." >&2
  exit 1
fi

LINES=$(wc -l < "$TMP_DIR/fleur-evm.jsonl" | tr -d ' ')
if [ "$LINES" -ne 3 ]; then
  echo "EVM journal should contain three replayable commands; found $LINES." >&2
  exit 1
fi

echo "Fleur persistent EVM JSON-RPC smoke test passed (return value 42)."
echo "Two RPC requests replayed through the separate EVM journal; commands are not yet block-committed."
