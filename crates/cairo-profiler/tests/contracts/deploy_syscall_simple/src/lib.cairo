use snforge_std::{declare, DeclareResultTrait};
use starknet::syscalls::deploy_syscall;

#[starknet::contract]
mod GasConstructorChecker {
    #[storage]
    struct Storage {}

    #[constructor]
    fn constructor(ref self: ContractState, _dummy_calldata: felt252) {
        core::keccak::keccak_u256s_le_inputs(array![1].span());
        core::keccak::keccak_u256s_le_inputs(array![1].span());
    }
}

#[test]
fn deploy_syscall_cost() {
    let contract = declare("GasConstructorChecker").unwrap().contract_class().clone();
    let (address, _) = deploy_syscall(contract.class_hash, 0, array![1].span(), false).unwrap();

    assert(address != 0.try_into().unwrap(), 'wrong deployed addr');
}
