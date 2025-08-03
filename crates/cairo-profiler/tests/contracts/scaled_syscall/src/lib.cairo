use snforge_std::{declare, DeclareResultTrait};
use starknet::syscalls::deploy_syscall;
use starknet::ContractAddress;

#[starknet::contract]
mod GasConstructorChecker {
    #[storage]
    struct Storage {}

    #[constructor]
    fn constructor(ref self: ContractState, _dummy_calldata1: felt252, _dummy_calldata2: felt252, _dummy_calldata3: felt252) {
        core::keccak::keccak_u256s_le_inputs(array![1].span());
        core::keccak::keccak_u256s_le_inputs(array![1].span());
    }
}

#[starknet::contract]
mod GasConstructorCheckerButDifferent {
    #[storage]
    struct Storage {}

    #[constructor]
    fn constructor(ref self: ContractState, _dummy_calldata: felt252) {
        core::keccak::keccak_u256s_le_inputs(array![1].span());
        core::keccak::keccak_u256s_le_inputs(array![1].span());
    }
}

fn declare_deploy_a_contract() {
    let contract1 = declare("GasConstructorCheckerButDifferent").unwrap().contract_class().clone();
    deploy_syscall(contract1.class_hash, 0, array![1].span(), false).unwrap();
}

#[test]
fn deploy_syscall_cost() {
    let contract = declare("GasConstructorChecker").unwrap().contract_class().clone();
    let (address, _) = deploy_syscall(contract.class_hash, 0, array![1, 2, 3].span(), false).unwrap();

    assert(address != 0.try_into().unwrap(), 'wrong deployed addr');

    declare_deploy_a_contract()
}

#[test]
fn deploy_syscall_cost_but_different() {
    let contract1 = declare("GasConstructorCheckerButDifferent").unwrap().contract_class().clone();
    let (address, _) = deploy_syscall(contract1.class_hash, 0, array![1].span(), false).unwrap();

    assert(address != 0.try_into().unwrap(), 'wrong deployed addr');
}

#[starknet::interface]
trait IHelloStarknet<TContractState> {
    fn increase_balance(ref self: TContractState, amount: felt252);
    fn get_balance(self: @TContractState) -> felt252;
}

#[test]
#[fork(url: "http://188.34.188.184:7070/rpc/v0_8", block_number: 947567)]
fn test_increase_balance() {
    let contract_address: ContractAddress =
        0x000fa8e78a86a612746455cfeb98012e67ec3426b41a20278d5e7237bcab7413
        .try_into()
        .unwrap();
    let dispatcher = IHelloStarknetDispatcher { contract_address };

    dispatcher.increase_balance(42);
}
