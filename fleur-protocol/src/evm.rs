//! Isolated REVM execution support for Fleur's EVM compatibility target.
//!
//! The sandbox API is stateless. A separate JSONL command journal can replay EVM
//! deployments/calls across requests and restarts, but it is not part of block
//! validation, the ledger, consensus, or an atomic persistent state database.

use revm::{
    context::{Context, TxEnv},
    context_interface::result::{ExecutionResult, Output},
    database::CacheDB,
    database_interface::EmptyDB,
    primitives::{Bytes, TxKind},
    ExecuteCommitEvm, MainBuilder, MainContext,
};

/// Summary from a contract deployment followed by one call in a fresh EVM sandbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvmExecutionReport {
    pub contract_address: String,
    pub deployment_gas_used: u64,
    pub call_gas_used: u64,
    pub return_data: String,
}

/// Deploy init bytecode, then call the resulting contract once.
///
/// This deliberately creates a fresh in-memory EVM for each invocation; state is
/// not persisted between RPC requests. The caller must enforce transaction policy
/// and account for gas before treating this as a chain transaction.
pub fn deploy_and_call(
    init_code: Vec<u8>,
    call_data: Vec<u8>,
) -> Result<EvmExecutionReport, String> {
    let context = Context::mainnet().with_db(CacheDB::<EmptyDB>::default());
    let mut evm = context.build_mainnet();
    let creation = evm
        .transact_commit(
            TxEnv::builder()
                .kind(TxKind::Create)
                .data(Bytes::from(init_code))
                .build()
                .map_err(|error| format!("invalid create transaction: {error:?}"))?,
        )
        .map_err(|error| format!("contract deployment failed: {error:?}"))?;

    let deployment_gas_used = creation.gas_used();
    let contract_address = match creation {
        ExecutionResult::Success {
            output: Output::Create(_, Some(address)),
            ..
        } => address,
        other => return Err(format!("contract deployment did not succeed: {other:?}")),
    };

    let call = evm
        .transact_commit(
            TxEnv::builder()
                .kind(TxKind::Call(contract_address))
                .data(Bytes::from(call_data))
                .nonce(1)
                .build()
                .map_err(|error| format!("invalid call transaction: {error:?}"))?,
        )
        .map_err(|error| format!("contract call failed: {error:?}"))?;

    let call_gas_used = call.gas_used();
    match call {
        ExecutionResult::Success {
            output: Output::Call(output),
            ..
        } => Ok(EvmExecutionReport {
            contract_address: format!("{contract_address:#x}"),
            deployment_gas_used,
            call_gas_used,
            return_data: hex_encode(&output),
        }),
        other => Err(format!("contract call did not succeed: {other:?}")),
    }
}

/// Kind of command stored in the replay journal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
enum EvmOperationKind {
    #[default]
    DeployAndCall,
    CallExisting,
}

/// One accepted EVM command stored in the replay journal.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct EvmOperation {
    #[serde(default)]
    kind: EvmOperationKind,
    #[serde(default)]
    init_code: Vec<u8>,
    #[serde(default)]
    contract_address: Option<String>,
    #[serde(default)]
    call_data: Vec<u8>,
}

/// Execute a deployment followed by a call and append it to the command journal.
///
/// The journal stores commands, not a serialized database snapshot. Every request
/// replays the command sequence against one EVM instance before appending the new
/// operation. This is still separate from Fleur block validation and gas settlement.
pub fn persistent_deploy_and_call(
    journal_path: impl AsRef<std::path::Path>,
    init_code: Vec<u8>,
    call_data: Vec<u8>,
) -> Result<EvmExecutionReport, String> {
    if init_code.is_empty() {
        return Err("init code must not be empty".to_string());
    }
    persist_evm_operation(
        journal_path,
        EvmOperation {
            kind: EvmOperationKind::DeployAndCall,
            init_code,
            contract_address: None,
            call_data,
        },
    )
}

/// Call an already deployed contract address by replaying the journal first.
///
/// The contract address must refer to a contract deployed by an earlier journal
/// operation. State changes are retained in the command sequence across requests
/// and restarts, but are not yet committed as part of Fleur blocks.
pub fn persistent_call_contract(
    journal_path: impl AsRef<std::path::Path>,
    contract_address: String,
    call_data: Vec<u8>,
) -> Result<EvmExecutionReport, String> {
    parse_address(&contract_address)?;
    persist_evm_operation(
        journal_path,
        EvmOperation {
            kind: EvmOperationKind::CallExisting,
            init_code: Vec::new(),
            contract_address: Some(contract_address),
            call_data,
        },
    )
}

fn persist_evm_operation(
    journal_path: impl AsRef<std::path::Path>,
    operation: EvmOperation,
) -> Result<EvmExecutionReport, String> {
    use std::io::Write;

    let path = journal_path.as_ref();
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("cannot read EVM journal: {error}")),
    };
    let mut operations = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let prior: EvmOperation = serde_json::from_str(line)
            .map_err(|error| format!("invalid EVM journal line {}: {error}", index + 1))?;
        validate_operation(&prior)
            .map_err(|error| format!("invalid EVM journal line {}: {error}", index + 1))?;
        operations.push(prior);
    }
    validate_operation(&operation)?;
    let mut candidate = operations;
    candidate.push(operation.clone());
    let reports = execute_operations(&candidate)?;
    let report = reports
        .last()
        .cloned()
        .ok_or_else(|| "EVM journal produced no report".to_string())?;

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create EVM journal directory: {error}"))?;
    }
    let encoded = serde_json::to_string(&operation)
        .map_err(|error| format!("cannot encode EVM operation: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("cannot open EVM journal: {error}"))?;
    writeln!(file, "{encoded}").map_err(|error| format!("cannot append EVM journal: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync EVM journal: {error}"))?;
    Ok(report)
}

fn validate_operation(operation: &EvmOperation) -> Result<(), String> {
    match operation.kind {
        EvmOperationKind::DeployAndCall if operation.init_code.is_empty() => {
            Err("deployment init code must not be empty".to_string())
        }
        EvmOperationKind::DeployAndCall => Ok(()),
        EvmOperationKind::CallExisting => {
            let address = operation
                .contract_address
                .as_deref()
                .ok_or_else(|| "existing-contract call requires contract address".to_string())?;
            parse_address(address).map(|_| ())
        }
    }
}

fn execute_operations(operations: &[EvmOperation]) -> Result<Vec<EvmExecutionReport>, String> {
    use revm::primitives::Address;

    let context = Context::mainnet().with_db(CacheDB::<EmptyDB>::default());
    let mut evm = context.build_mainnet();
    let mut nonce = 0u64;
    let mut reports = Vec::with_capacity(operations.len());

    for operation in operations {
        let (contract_address, deployment_gas_used) = match operation.kind {
            EvmOperationKind::DeployAndCall => {
                let creation = evm
                    .transact_commit(
                        TxEnv::builder()
                            .kind(TxKind::Create)
                            .data(Bytes::from(operation.init_code.clone()))
                            .nonce(nonce)
                            .build()
                            .map_err(|error| format!("invalid create transaction: {error:?}"))?,
                    )
                    .map_err(|error| format!("contract deployment failed: {error:?}"))?;
                let gas_used = creation.gas_used();
                let address = match creation {
                    ExecutionResult::Success {
                        output: Output::Create(_, Some(address)),
                        ..
                    } => address,
                    other => {
                        return Err(format!("contract deployment did not succeed: {other:?}"))
                    }
                };
                nonce = nonce
                    .checked_add(1)
                    .ok_or_else(|| "EVM transaction nonce overflow".to_string())?;
                (address, gas_used)
            }
            EvmOperationKind::CallExisting => {
                let address = operation
                    .contract_address
                    .as_deref()
                    .ok_or_else(|| "existing-contract call requires contract address".to_string())?;
                (parse_address(address)?, 0)
            }
        };

        let call = evm
            .transact_commit(
                TxEnv::builder()
                    .kind(TxKind::Call(contract_address))
                    .data(Bytes::from(operation.call_data.clone()))
                    .nonce(nonce)
                    .build()
                    .map_err(|error| format!("invalid call transaction: {error:?}"))?,
            )
            .map_err(|error| format!("contract call failed: {error:?}"))?;
        let call_gas_used = call.gas_used();
        let return_data = match call {
            ExecutionResult::Success {
                output: Output::Call(output),
                ..
            } => hex_encode(&output),
            other => return Err(format!("contract call did not succeed: {other:?}")),
        };
        nonce = nonce
            .checked_add(1)
            .ok_or_else(|| "EVM transaction nonce overflow".to_string())?;
        reports.push(EvmExecutionReport {
            contract_address: format!("{contract_address:#x}"),
            deployment_gas_used,
            call_gas_used,
            return_data,
        });
    }
    Ok(reports)
}

fn parse_address(value: &str) -> Result<revm::primitives::Address, String> {
    let bytes = decode_hex(value).map_err(|_| "contract address must be hex".to_string())?;
    if bytes.len() != 20 {
        return Err("contract address must contain exactly 20 bytes".to_string());
    }
    Ok(revm::primitives::Address::from_slice(&bytes))
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
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use revm::{
        bytecode::opcode, context_interface::result::Output, primitives::StorageValue, ExecuteEvm,
    };

    const INIT_CODE: &[u8] = &[
        opcode::PUSH1,
        0x01,
        opcode::PUSH1,
        0x17,
        opcode::PUSH1,
        0x1f,
        opcode::CODECOPY,
        opcode::PUSH0,
        opcode::MLOAD,
        opcode::PUSH0,
        opcode::SSTORE,
    ];
    const RETURN_RUNTIME: &[u8] = &[
        opcode::PUSH1,
        0x02,
        opcode::PUSH1,
        0x15,
        opcode::PUSH0,
        opcode::CODECOPY,
        opcode::PUSH1,
        0x02,
        opcode::PUSH0,
        opcode::RETURN,
    ];
    const RUNTIME_BYTECODE: &[u8] = &[opcode::PUSH0, opcode::SLOAD];

    #[test]
    fn revm_deploys_contract_and_executes_storage_write() {
        let expected_value = 0x42u8;
        let bytecode: Bytes = [
            INIT_CODE,
            RETURN_RUNTIME,
            RUNTIME_BYTECODE,
            &[expected_value],
        ]
        .concat()
        .into();
        let context = Context::mainnet().with_db(CacheDB::<EmptyDB>::default());
        let mut evm = context.build_mainnet();

        let creation = evm
            .transact_commit(
                TxEnv::builder()
                    .kind(TxKind::Create)
                    .data(bytecode)
                    .build()
                    .unwrap(),
            )
            .expect("REVM should execute contract creation");

        let contract_address = match creation {
            ExecutionResult::Success {
                output: Output::Create(_, Some(address)),
                ..
            } => address,
            other => panic!("contract creation failed: {other:#?}"),
        };

        let call = evm
            .transact(
                TxEnv::builder()
                    .kind(TxKind::Call(contract_address))
                    .data(Default::default())
                    .nonce(1)
                    .build()
                    .unwrap(),
            )
            .expect("REVM should execute contract call");

        assert!(matches!(call.result, ExecutionResult::Success { .. }));
        let account = call
            .state
            .get(&contract_address)
            .expect("deployed contract must exist");
        let storage = account
            .storage
            .get::<StorageValue>(&Default::default())
            .expect("contract must have slot-zero storage");
        assert_eq!(
            storage.present_value(),
            StorageValue::try_from(expected_value).unwrap()
        );
    }

    #[test]
    fn persistent_evm_journal_replays_prior_deployments_after_restart() {
        let path =
            std::env::temp_dir().join(format!("fleur-evm-journal-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let init_code = vec![
            0x60, 0x0a, 0x60, 0x0c, 0x60, 0x00, 0x39, 0x60, 0x0a, 0x60, 0x00, 0xf3, 0x60, 0x2a,
            0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3,
        ];

        let first = persistent_deploy_and_call(&path, init_code.clone(), Vec::new()).unwrap();
        assert_eq!(first.return_data, format!("{:064x}", 42));
        let second = persistent_deploy_and_call(&path, init_code.clone(), Vec::new()).unwrap();
        assert_eq!(second.return_data, format!("{:064x}", 42));
        assert_ne!(first.contract_address, second.contract_address);

        let lines = std::fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 2);
        let after_restart = persistent_deploy_and_call(&path, init_code, Vec::new()).unwrap();
        assert_eq!(after_restart.return_data, format!("{:064x}", 42));
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn deployment_and_call_report_gas_and_return_data() {
        // Init code returns runtime bytecode: PUSH1 0x2a PUSH1 0 MSTORE
        // PUSH1 0x20 PUSH1 0 RETURN.
        let init_code = vec![
            0x60, 0x0a, 0x60, 0x0c, 0x60, 0x00, 0x39, 0x60, 0x0a, 0x60, 0x00, 0xf3, 0x60, 0x2a,
            0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3,
        ];
        let report = deploy_and_call(init_code, Vec::new()).unwrap();
        assert!(report.contract_address.starts_with("0x"));
        assert!(report.deployment_gas_used > 0);
        assert!(report.call_gas_used > 0);
        assert_eq!(report.return_data, format!("{:064x}", 42));
    }
}
