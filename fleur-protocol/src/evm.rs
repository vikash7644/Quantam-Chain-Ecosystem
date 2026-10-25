//! REVM-backed execution smoke test for Fleur's EVM compatibility target.
//!
//! This module proves the pinned EVM engine can execute deployment and contract
//! bytecode. It is not yet wired into Fleur block validation, RPC, gas accounting,
//! or persistent chain state.

#[cfg(test)]
mod tests {
    use revm::{
        bytecode::opcode,
        context::{Context, TxEnv},
        context_interface::result::{ExecutionResult, Output},
        database::CacheDB,
        database_interface::EmptyDB,
        primitives::{Bytes, StorageValue, TxKind},
        ExecuteCommitEvm, ExecuteEvm, MainBuilder, MainContext,
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
}
