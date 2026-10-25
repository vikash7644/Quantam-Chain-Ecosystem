use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use fleur_protocol::{Block, LedgerState, PersistentChain, SignedTransaction};
use serde_json::{json, Value};
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

type SharedChain = Arc<Mutex<PersistentChain>>;

fn parse_genesis_balance(value: &str) -> Result<u128, std::num::ParseIntError> {
    value.parse::<u128>()
}

fn parse_chain_id(value: &str) -> Result<u64, std::num::ParseIntError> {
    value.parse::<u64>()
}

fn build_genesis(address: String, balance: u128) -> Result<LedgerState, &'static str> {
    if address.is_empty() {
        if balance != 0 {
            return Err("FLEUR_GENESIS_BALANCE requires FLEUR_GENESIS_ADDRESS");
        }
        return Ok(LedgerState::default());
    }
    Ok(LedgerState::with_genesis([(address, balance)]))
}

#[tokio::main]
async fn main() {
    let journal_path = env::var_os("FLEUR_JOURNAL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data/fleur-chain.jsonl"));
    let genesis_address = env::var("FLEUR_GENESIS_ADDRESS").unwrap_or_default();
    let genesis_balance = match env::var("FLEUR_GENESIS_BALANCE") {
        Ok(value) => parse_genesis_balance(&value)
            .expect("FLEUR_GENESIS_BALANCE must be a valid unsigned integer"),
        Err(env::VarError::NotPresent) => 0,
        Err(error) => panic!("Unable to read FLEUR_GENESIS_BALANCE: {error}"),
    };
    let genesis =
        build_genesis(genesis_address, genesis_balance).expect("invalid genesis configuration");
    let chain = PersistentChain::open(&journal_path, genesis, "FLEUR_GENESIS_V1")
        .expect("journal must replay successfully");
    let shared = Arc::new(Mutex::new(chain));

    let app = Router::new()
        .route("/health", get(handle_health))
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
    println!("Development endpoint only: no consensus, peer networking, or persistent EVM state.");
    axum::serve(listener, app).await.expect("serve Fleur RPC");
}

async fn handle_health(State(chain): State<SharedChain>) -> Json<Value> {
    match chain.lock() {
        Ok(chain) => Json(json!({
            "status": "ok",
            "service": "fleur-node",
            "height": chain.chain.tip_height
        })),
        Err(_) => Json(json!({
            "status": "error",
            "service": "fleur-node",
            "reason": "chain state lock poisoned"
        })),
    }
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
        "eth_chainId" => {
            // A development-only chain ID for tooling smoke tests. Operators may
            // override it with a decimal FLEUR_CHAIN_ID value.
            let chain_id = match env::var("FLEUR_CHAIN_ID") {
                Ok(value) => match parse_chain_id(&value) {
                    Ok(chain_id) => chain_id,
                    Err(_) => {
                        return rpc_error(
                            id,
                            -32603,
                            "FLEUR_CHAIN_ID must be a valid unsigned integer",
                        )
                    }
                },
                Err(env::VarError::NotPresent) => 61800,
                Err(_) => return rpc_error(id, -32603, "Unable to read FLEUR_CHAIN_ID"),
            };
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": format!("0x{chain_id:x}")
            }))
        }
        "eth_blockNumber" => {
            let Ok(chain) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": format!("0x{:x}", chain.chain.tip_height)
            }))
        }
        "fleur_nativeCoin" => {
            // Identifies the planned native asset. This metadata does not imply
            // a finalized supply policy, EVM gas settlement, or public launch.
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "name": "Fleur Network Native Coin",
                    "symbol": "BVRX",
                    "assetType": "native",
                    "unit": "base-unit",
                    "amountEncoding": "decimal-string",
                    "supplyPolicy": "not-finalized",
                    "networkStatus": "development-only"
                }
            }))
        }
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
                    "totalFees": chain.chain.ledger.total_fees.to_string(),
                    "totalFeesAsset": "BVRX",
                    "totalFeesUnit": "base-unit",
                    "totalFeesEncoding": "decimal-string"
                }
            }))
        }
        "fleur_getBlockByHeight" => {
            let Some(height) = params.get("height").and_then(Value::as_u64) else {
                return rpc_error(id, -32602, "params.height must be an unsigned integer");
            };
            if height == 0 {
                return rpc_error(
                    id,
                    -32602,
                    "genesis is configuration-defined; request height >= 1",
                );
            }
            let journal_path = env::var_os("FLEUR_JOURNAL")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./data/fleur-chain.jsonl"));
            let contents = match std::fs::read_to_string(&journal_path) {
                Ok(contents) => contents,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(_) => return rpc_error(id, -32020, "Unable to read block journal"),
            };
            for (index, line) in contents.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let block: Block = match serde_json::from_str(line) {
                    Ok(block) => block,
                    Err(_) => {
                        return rpc_error(
                            id,
                            -32020,
                            &format!("Invalid block journal entry at line {}", index + 1),
                        )
                    }
                };
                if block.height == height {
                    let block_hash = match block.hash() {
                        Ok(hash) => hash,
                        Err(_) => return rpc_error(id, -32020, "Unable to hash requested block"),
                    };
                    return Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "height": block.height,
                            "hash": block_hash,
                            "parentHash": block.previous_hash,
                            "transactions": block.transactions
                        }
                    }));
                }
            }
            Json(json!({ "jsonrpc": "2.0", "id": id, "result": null }))
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
                "result": {
                    "address": address,
                    "balance": chain.chain.ledger.balance(address).to_string(),
                    "asset": "BVRX",
                    "unit": "base-unit",
                    "amountEncoding": "decimal-string"
                }
            }))
        }
        "fleur_submitBlock" => {
            let Some(value) = params.get("block").cloned() else {
                return rpc_error(id, -32602, "params.block is required");
            };
            let block: Block = match serde_json::from_value(value) {
                Ok(block) => block,
                Err(_) => return rpc_error(id, -32602, "Invalid block shape"),
            };
            let Ok(mut chain) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            match chain.append_block(&block) {
                Ok(hash) => Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "height": block.height, "blockHash": hash }
                })),
                Err(error) => rpc_error(id, -32000, &format!("{error:?}")),
            }
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
        "fleur_evmPersistentDeployAndCall" => {
            let Some(init_code) = params.get("initCode").and_then(Value::as_str) else {
                return rpc_error(id, -32602, "params.initCode must be a hex string");
            };
            let init_code = match decode_hex(init_code) {
                Ok(bytes) if !bytes.is_empty() => bytes,
                _ => return rpc_error(id, -32602, "params.initCode must be non-empty valid hex"),
            };
            let call_data = match params.get("callData").and_then(Value::as_str) {
                Some(encoded) => match decode_hex(encoded) {
                    Ok(bytes) => bytes,
                    Err(_) => return rpc_error(id, -32602, "params.callData must be valid hex"),
                },
                None => Vec::new(),
            };
            // Serialize EVM journal updates with other node operations. The EVM journal
            // is durable/replayable but is not yet part of block consensus or gas settlement.
            let Ok(_chain_guard) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            let journal_path = env::var_os("FLEUR_EVM_JOURNAL")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./data/fleur-evm.jsonl"));
            match fleur_protocol::evm::persistent_deploy_and_call(
                journal_path,
                init_code,
                call_data,
            ) {
                Ok(report) => Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "mode": "persistent-command-replay",
                        "contractAddress": report.contract_address,
                        "deploymentGasUsed": report.deployment_gas_used.to_string(),
                        "callGasUsed": report.call_gas_used.to_string(),
                        "returnData": report.return_data
                    }
                })),
                Err(error) => rpc_error(id, -32011, &error),
            }
        }
        "fleur_evmPersistentCall" => {
            let Some(contract_address) = params.get("contractAddress").and_then(Value::as_str)
            else {
                return rpc_error(id, -32602, "params.contractAddress must be a hex address");
            };
            let call_data = match params.get("callData").and_then(Value::as_str) {
                Some(encoded) => match decode_hex(encoded) {
                    Ok(bytes) => bytes,
                    Err(_) => return rpc_error(id, -32602, "params.callData must be valid hex"),
                },
                None => Vec::new(),
            };
            // Calls replay all earlier EVM commands first. This is durable command
            // replay only; it is not yet included in block consensus or gas settlement.
            let Ok(_chain_guard) = chain.lock() else {
                return rpc_error(id, -32603, "Chain state lock poisoned");
            };
            let journal_path = env::var_os("FLEUR_EVM_JOURNAL")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./data/fleur-evm.jsonl"));
            match fleur_protocol::evm::persistent_call_contract(
                journal_path,
                contract_address.to_string(),
                call_data,
            ) {
                Ok(report) => Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "mode": "persistent-command-replay",
                        "contractAddress": report.contract_address,
                        "deploymentGasUsed": report.deployment_gas_used.to_string(),
                        "callGasUsed": report.call_gas_used.to_string(),
                        "returnData": report.return_data
                    }
                })),
                Err(error) => rpc_error(id, -32012, &error),
            }
        }
        "fleur_evmSandboxDeployAndCall" => {
            let Some(init_code) = params.get("initCode").and_then(Value::as_str) else {
                return rpc_error(id, -32602, "params.initCode must be a hex string");
            };
            let init_code = match decode_hex(init_code) {
                Ok(bytes) if !bytes.is_empty() => bytes,
                _ => return rpc_error(id, -32602, "params.initCode must be non-empty valid hex"),
            };
            let call_data = match params.get("callData").and_then(Value::as_str) {
                Some(encoded) => match decode_hex(encoded) {
                    Ok(bytes) => bytes,
                    Err(_) => return rpc_error(id, -32602, "params.callData must be valid hex"),
                },
                None => Vec::new(),
            };
            match fleur_protocol::evm::deploy_and_call(init_code, call_data) {
                Ok(report) => Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "mode": "isolated-in-memory-sandbox",
                        "contractAddress": report.contract_address,
                        "deploymentGasUsed": report.deployment_gas_used.to_string(),
                        "callGasUsed": report.call_gas_used.to_string(),
                        "returnData": report.return_data
                    }
                })),
                Err(error) => rpc_error(id, -32010, &error),
            }
        }
        _ => rpc_error(id, -32601, "Method not found"),
    }
}

fn decode_hex(input: &str) -> Result<Vec<u8>, ()> {
    let input = input.strip_prefix("0x").unwrap_or(input);
    if input.len() % 2 != 0 {
        return Err(());
    }
    (0..input.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&input[index..index + 2], 16).map_err(|_| ()))
        .collect()
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
    async fn balance_rpc_labels_amount_as_bvrx_base_units() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 18,
                "method": "fleur_getBalance",
                "params": { "address": "flr_test" }
            })),
        )
        .await;
        assert_eq!(response.0["result"]["address"], "flr_test");
        assert_eq!(response.0["result"]["balance"], "0");
        assert_eq!(response.0["result"]["asset"], "BVRX");
        assert_eq!(response.0["result"]["unit"], "base-unit");
        assert_eq!(response.0["result"]["amountEncoding"], "decimal-string");
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
        assert_eq!(response.0["result"]["totalFees"], "0");
        assert_eq!(response.0["result"]["totalFeesAsset"], "BVRX");
        assert_eq!(response.0["result"]["totalFeesUnit"], "base-unit");
        assert_eq!(response.0["result"]["totalFeesEncoding"], "decimal-string");
    }

    #[tokio::test]
    async fn block_lookup_returns_persisted_block_by_height() {
        use ed25519_dalek::{Signer, SigningKey};

        let path =
            std::env::temp_dir().join(format!("fleur-rpc-lookup-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let signing_key = SigningKey::from_bytes(&[13u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let sender = fleur_protocol::address_from_public_key(&public_key);
        let tx = fleur_protocol::Transaction {
            sender: sender.clone(),
            recipient: "flr_lookup_recipient".to_string(),
            amount: 5,
            fee: 1,
            nonce: 0,
        };
        let signature = signing_key.sign(&tx.signing_bytes().unwrap());
        let block = Block {
            height: 1,
            previous_hash: "FLEUR_GENESIS_V1".to_string(),
            transactions: vec![SignedTransaction {
                transaction: tx,
                public_key: hex_encode(&public_key),
                signature: hex_encode(&signature.to_bytes()),
            }],
        };
        let mut persistent = PersistentChain::open(
            &path,
            LedgerState::with_genesis([(sender, 20)]),
            "FLEUR_GENESIS_V1",
        )
        .unwrap();
        persistent.append_block(&block).unwrap();
        std::env::set_var("FLEUR_JOURNAL", &path);
        let response = handle_rpc(
            State(Arc::new(Mutex::new(persistent))),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "fleur_getBlockByHeight",
                "params": { "height": 1 }
            })),
        )
        .await;
        std::env::remove_var("FLEUR_JOURNAL");
        assert_eq!(response.0["result"]["height"], 1);
        assert_eq!(
            response.0["result"]["transactions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn submit_block_rpc_accepts_atomic_multi_transaction_block() {
        use ed25519_dalek::{Signer, SigningKey};

        let path =
            std::env::temp_dir().join(format!("fleur-rpc-block-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let signing_key = SigningKey::from_bytes(&[12u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let sender = fleur_protocol::address_from_public_key(&public_key);
        let genesis = LedgerState::with_genesis([(sender.clone(), 100)]);
        let persistent = PersistentChain::open(&path, genesis, "FLEUR_GENESIS_V1").unwrap();
        let shared = Arc::new(Mutex::new(persistent));

        let make_signed = |nonce: u64, recipient: &str, amount: u128| {
            let transaction = fleur_protocol::Transaction {
                sender: sender.clone(),
                recipient: recipient.to_string(),
                amount,
                fee: 1,
                nonce,
            };
            let signature = signing_key.sign(&transaction.signing_bytes().unwrap());
            SignedTransaction {
                transaction,
                public_key: hex_encode(&public_key),
                signature: hex_encode(&signature.to_bytes()),
            }
        };
        let block = Block {
            height: 1,
            previous_hash: "FLEUR_GENESIS_V1".to_string(),
            transactions: vec![
                make_signed(0, "flr_first", 10),
                make_signed(1, "flr_second", 20),
            ],
        };
        let response = handle_rpc(
            State(shared.clone()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "fleur_submitBlock",
                "params": { "block": block }
            })),
        )
        .await;
        assert_eq!(response.0["result"]["height"], 1);
        let chain = shared.lock().unwrap();
        assert_eq!(chain.chain.ledger.balance("flr_first"), 10);
        assert_eq!(chain.chain.ledger.balance("flr_second"), 20);
        assert_eq!(chain.chain.ledger.balance(&sender), 68);
        assert_eq!(chain.chain.ledger.total_fees, 2);
        drop(chain);
        let _ = std::fs::remove_file(path);
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

    #[test]
    fn genesis_configuration_rejects_funded_balance_without_address() {
        assert!(build_genesis(String::new(), 1).is_err());
        assert!(build_genesis(String::new(), 0).is_ok());
        assert!(build_genesis("flr_genesis".to_string(), 1).is_ok());
    }

    #[test]
    fn configuration_parsers_accept_valid_unsigned_values_and_reject_invalid_ones() {
        assert_eq!(parse_genesis_balance("0").unwrap(), 0);
        assert_eq!(
            parse_genesis_balance("340282366920938463463374607431768211455").unwrap(),
            u128::MAX
        );
        assert!(parse_genesis_balance("-1").is_err());
        assert!(parse_genesis_balance("1.5").is_err());
        assert!(parse_genesis_balance("340282366920938463463374607431768211456").is_err());
        assert_eq!(parse_chain_id("61800").unwrap(), 61800);
        assert_eq!(parse_chain_id("18446744073709551615").unwrap(), u64::MAX);
        assert!(parse_chain_id("-1").is_err());
        assert!(parse_chain_id("abc").is_err());
        assert!(parse_chain_id("18446744073709551616").is_err());
    }

    #[tokio::test]
    #[tokio::test]
    async fn native_coin_rpc_reports_bvrx_without_claiming_final_supply() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 17,
                "method": "fleur_nativeCoin"
            })),
        )
        .await;
        assert_eq!(response.0["result"]["name"], "Fleur Network Native Coin");
        assert_eq!(response.0["result"]["symbol"], "BVRX");
        assert_eq!(response.0["result"]["assetType"], "native");
        assert_eq!(response.0["result"]["supplyPolicy"], "not-finalized");
        assert_eq!(response.0["result"]["networkStatus"], "development-only");
    }

    async fn health_handler_reports_chain_height() {
        let response = handle_health(State(test_chain())).await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["service"], "fleur-node");
        assert_eq!(response.0["height"], 0);
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

    #[tokio::test]
    async fn evm_sandbox_rpc_rejects_malformed_init_code() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "fleur_evmSandboxDeployAndCall",
                "params": { "initCode": "0xzz" }
            })),
        )
        .await;
        assert_eq!(response.0["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn evm_sandbox_rpc_deploys_and_calls_contract() {
        let response = handle_rpc(
            State(test_chain()),
            Json(json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "fleur_evmSandboxDeployAndCall",
                "params": {
                    "initCode": "0x600a600c600039600a6000f3602a60005260206000f3"
                }
            })),
        )
        .await;

        assert_eq!(response.0["result"]["mode"], "isolated-in-memory-sandbox");
        assert!(response.0["result"]["contractAddress"]
            .as_str()
            .is_some_and(|address| address.starts_with("0x")));
        assert_eq!(response.0["result"]["returnData"], format!("{:064x}", 42));
        assert!(response.0["result"]["deploymentGasUsed"]
            .as_str()
            .and_then(|gas| gas.parse::<u64>().ok())
            .is_some_and(|gas| gas > 0));
        assert!(response.0["result"]["callGasUsed"]
            .as_str()
            .and_then(|gas| gas.parse::<u64>().ok())
            .is_some_and(|gas| gas > 0));
    }
}
