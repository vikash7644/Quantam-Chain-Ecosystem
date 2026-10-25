//! Isolated REVM execution support for Fleur's EVM compatibility target.
//!
//! Execution currently runs in a fresh in-memory sandbox. It is not connected to
//! block validation, the ledger, consensus, or persistent contract storage.

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
