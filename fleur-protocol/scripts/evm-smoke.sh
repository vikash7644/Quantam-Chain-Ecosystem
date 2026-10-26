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

# Verify BVRX is clearly labeled as a planned development asset and that
# accumulated fees use the same explicit base-unit encoding.
if ! grep -q '"totalFees":"0"' "$TMP_DIR/status.json" ||
   ! grep -q '"totalFeesAsset":"BVRX"' "$TMP_DIR/status.json" ||
   ! grep -q '"totalFeesUnit":"base-unit"' "$TMP_DIR/status.json" ||
   ! grep -q '"totalFeesEncoding":"decimal-string"' "$TMP_DIR/status.json"; then
  cat "$TMP_DIR/status.json"
  echo "Chain status did not expose explicit BVRX fee metadata." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":14,"method":"fleur_nativeCoin"}' \
  >"$TMP_DIR/native-coin.json"
if ! grep -q '"symbol":"BVRX"' "$TMP_DIR/native-coin.json" ||
   ! grep -q '"supplyPolicy":"not-finalized"' "$TMP_DIR/native-coin.json" ||
   ! grep -q '"networkStatus":"development-only"' "$TMP_DIR/native-coin.json"; then
  cat "$TMP_DIR/native-coin.json"
  echo "Native-coin metadata did not preserve development-only caveats." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/health" >"$TMP_DIR/health.json"
if ! grep -q '"status":"ok"' "$TMP_DIR/health.json"; then
  cat "$TMP_DIR/health.json"
  echo "Node health endpoint did not report ok." >&2
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
  -d '{"jsonrpc":"2.0","id":2,"method":"fleur_evmPersistentDeployAndCall","params":{"initCode":"0x600e600c600039600e6000f35f54600101805f555f5260205ff3"}}' \
  >"$TMP_DIR/evm.json"

if ! grep -q '"returnData":"0000000000000000000000000000000000000000000000000000000000000001"' "$TMP_DIR/evm.json"; then
  cat "$TMP_DIR/evm.json"
  echo "EVM smoke test failed: expected initial counter value 1." >&2
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

if ! grep -q '"returnData":"0000000000000000000000000000000000000000000000000000000000000002"' "$TMP_DIR/evm-existing-call.json"; then
  cat "$TMP_DIR/evm-existing-call.json"
  echo "EVM existing-contract call failed: expected counter value 2." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":13,\"method\":\"fleur_evmPersistentCall\",\"params\":{\"contractAddress\":\"$ADDRESS\"}}" \
  >"$TMP_DIR/evm-existing-call-second.json"

if ! grep -q '"returnData":"0000000000000000000000000000000000000000000000000000000000000003"' "$TMP_DIR/evm-existing-call-second.json"; then
  cat "$TMP_DIR/evm-existing-call-second.json"
  echo "EVM storage replay failed: expected counter value 3." >&2
  exit 1
fi

curl -fsS "http://127.0.0.1:$PORT/rpc" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":3,"method":"fleur_evmPersistentDeployAndCall","params":{"initCode":"0x600e600c600039600e6000f35f54600101805f555f5260205ff3"}}' \
  >"$TMP_DIR/evm-second.json"

if ! grep -q '"returnData":"0000000000000000000000000000000000000000000000000000000000000001"' "$TMP_DIR/evm-second.json"; then
  cat "$TMP_DIR/evm-second.json"
  echo "Second deployment should start at counter value 1." >&2
  exit 1
fi

LINES=$(wc -l < "$TMP_DIR/fleur-evm.jsonl" | tr -d ' ')
if [ "$LINES" -ne 4 ]; then
  echo "EVM journal should contain four replayable commands; found $LINES." >&2
  exit 1
fi

echo "Fleur persistent EVM JSON-RPC smoke test passed (return value 42)."
echo "Two RPC requests replayed through the separate EVM journal; commands are not yet block-committed."
