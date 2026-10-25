use axum::{extract::State, routing::post, Json, Router};
use fleur_protocol::{Block, LedgerState, PersistentChain, SignedTransaction};
use serde_json::{json, Value};
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

type SharedChain = Arc<Mutex<PersistentChain>>;

#[tokio::main]
async fn main() {
    let journal_path = env::var_os("FLEUR_JOURNAL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data/fleur-chain.jsonl"));
    let genesis_address = env::var("FLEUR_GENESIS_ADDRESS").unwrap_or_default();
    let genesis_balance = env::var("FLEUR_GENESIS_BALANCE")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or_default();
    let genesis = if genesis_address.is_empty() {
        LedgerState::default()
    } else {
        LedgerState::with_genesis([(genesis_address, genesis_balance)])
    };
    let chain = PersistentChain::open(&journal_path, genesis, "FLEUR_GENESIS_V1")
        .expect("journal must replay successfully");
    let shared = Arc::new(Mutex::new(chain));

    let app = Router::new()
        .route("/rpc", post(handle_rpc))
        .with_state(shared);
    let address: SocketAddr = env::var("FLEUR_RPC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8545".to_string())
        .parse()
        .expect("FLEUR_RPC_ADDR must be a socket address");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind Fleur RPC listener");
    println!("Fleur single-node dev RPC listening at http://{address}/rpc");
    println!("This is a development endpoint; consensus and EVM execution are not implemented.");
    axum::serve(listener, app).await.expect("serve Fleur RPC");
}

async fn handle_rpc(State(chain): State<SharedChain>, Json(request): Json<Value>) -> Json<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return rpc_error(id, -32600, "Invalid JSON-RPC request");
    }
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return rpc_error(id, -32600, "Missing method");
    };
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "fleur_chainStatus" => {
            let Ok(chain) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "height": chain.chain.tip_height,
                    "blockHash": chain.chain.tip_hash,
                    "totalFees": chain.chain.ledger.total_fees.to_string()
                }
            }))
        }
        "fleur_getBalance" => {
            let Some(address) = params.get("address").and_then(Value::as_str) else {
                return rpc_error(id, -32602, "params.address must be a string");
            };
            let Ok(chain) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "address": address, "balance": chain.chain.ledger.balance(address).to_string() }
            }))
        }
        "fleur_submitTransaction" => {
            let Some(value) = params.get("signedTransaction").cloned() else {
                return rpc_error(id, -32602, "params.signedTransaction is required");
            };
            let signed: SignedTransaction = match serde_json::from_value(value) {
                Ok(transaction) => transaction,
                Err(_) => return rpc_error(id, -32602, "Invalid signed transaction shape"),
            };
            let Ok(mut chain) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            let Some(height) = chain.chain.tip_height.checked_add(1) else {
                return rpc_error(id, -32000, "Block height overflow");
            };
            let block = Block {
                height,
                previous_hash: chain.chain.tip_hash.clone(),
                transactions: vec![signed],
            };
            match chain.append_block(&block) {
                Ok(hash) => Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "height": height, "blockHash": hash }
                })),
                Err(error) => rpc_error(id, -32000, &format!("{error:?}")),
            }
        }
        _ => rpc_error(id, -32601, "Method not found"),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn rpc_error(id: Value, code: i64, message: &str) -> Json<Value> {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chain() -> SharedChain {
        let path =
            std::env::temp_dir().join(format!("fleur-rpc-test-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let chain = PersistentChain::open(&path, LedgerState::default(), "FLEUR_GENESIS_V1")
            .expect("open test journal");
        Arc::new(Mutex::new(chain))
    }

    #[tokio::test]
    async fn chain_status_rpc_returns_genesis_tip() {
        let chain = test_chain();
        let response = handle_rpc(
            State(chain),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "fleur_chainStatus"
            })),
        )
        .await;
        assert_eq!(response.0["result"]["height"], 0);
        assert_eq!(response.0["result"]["blockHash"], "FLEUR_GENESIS_V1");
    }

    #[tokio::test]
    async fn submit_transaction_rpc_verifies_and_persists_signed_transfer() {
        use ed25519_dalek::{Signer, SigningKey};

        let path =
            std::env::temp_dir().join(format!("fleur-rpc-submit-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let sender = fleur_protocol::address_from_public_key(&public_key);
        let recipient = "flr_rpc_recipient".to_string();
        let ledger = LedgerState::with_genesis([(sender.clone(), 100)]);
        let chain = PersistentChain::open(&path, ledger, "FLEUR_GENESIS_V1").unwrap();
        let shared = Arc::new(Mutex::new(chain));
        let transaction = fleur_protocol::Transaction {
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount: 10,
            fee: 1,
            nonce: 0,
        };
        let signature = signing_key.sign(&transaction.signing_bytes().unwrap());
        let signed = SignedTransaction {
            transaction,
            public_key: hex_encode(&public_key),
            signature: hex_encode(&signature.to_bytes()),
        };

        let response = handle_rpc(
            State(shared.clone()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "fleur_submitTransaction",
                "params": { "signedTransaction": signed }
            })),
        )
        .await;
        assert_eq!(response.0["result"]["height"], 1);
        {
            let chain = shared.lock().unwrap();
            assert_eq!(chain.chain.ledger.balance(&sender), 89);
            assert_eq!(chain.chain.ledger.balance(&recipient), 10);
            assert_eq!(chain.chain.ledger.total_fees, 1);
        }
        drop(shared);
        let replayed = PersistentChain::open(
            &path,
            LedgerState::with_genesis([(sender.clone(), 100)]),
            "FLEUR_GENESIS_V1",
        )
        .expect("reopen journal and replay signed transfer");
        assert_eq!(replayed.chain.tip_height, 1);
        assert_eq!(replayed.chain.ledger.balance(&sender), 89);
        assert_eq!(replayed.chain.ledger.balance(&recipient), 10);
        assert_eq!(replayed.chain.ledger.total_fees, 1);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn invalid_signature_rpc_is_rejected_without_advancing_chain() {
        let path =
            std::env::temp_dir().join(format!("fleur-rpc-invalid-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let chain = PersistentChain::open(&path, LedgerState::default(), "FLEUR_GENESIS_V1")
            .expect("open test journal");
        let shared = Arc::new(Mutex::new(chain));
        let forged = SignedTransaction {
            transaction: fleur_protocol::Transaction {
                sender: "flr_forged".to_string(),
                recipient: "flr_target".to_string(),
                amount: 10,
                fee: 1,
                nonce: 0,
            },
            public_key: "00".repeat(32),
            signature: "00".repeat(64),
        };

        let response = handle_rpc(
            State(shared.clone()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "fleur_submitTransaction",
                "params": { "signedTransaction": forged }
            })),
        )
        .await;

        assert_eq!(response.0["error"]["code"], -32000);
        {
            let chain = shared.lock().unwrap();
            assert_eq!(chain.chain.tip_height, 0);
            assert_eq!(chain.chain.ledger.balance("flr_target"), 0);
        }
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn invalid_json_rpc_version_is_rejected() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "1.0",
                "id": 5,
                "method": "fleur_chainStatus"
            })),
        )
        .await;
        assert_eq!(response.0["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn unknown_rpc_method_returns_method_not_found() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "fleur_unknownMethod"
            })),
        )
        .await;
        assert_eq!(response.0["error"]["code"], -32601);
    }
}
