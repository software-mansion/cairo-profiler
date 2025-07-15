use balance_simple::{IHelloStarknetSafeDispatcher, IHelloStarknetSafeDispatcherTrait};
use snforge_std::{ContractClassTrait, DeclareResultTrait, declare};
use starknet::ContractAddress;

fn deploy_contract(name: ByteArray) -> ContractAddress {
    let contract = declare(name).unwrap().contract_class();
    let (contract_address, _) = contract.deploy(@ArrayTrait::new()).unwrap();
    contract_address
}

#[test]
#[feature("safe_dispatcher")]
fn test_cannot_increase_balance_with_zero_value() {
    let contract_address = deploy_contract("HelloStarknet");

    let safe_dispatcher = IHelloStarknetSafeDispatcher { contract_address };

    let balance_before = safe_dispatcher.get_balance().unwrap();
    assert(balance_before == 0, 'Invalid balance');
}

#[test]
#[fork(url: "http://188.34.188.184:7070/rpc/v0_8", block_number: 997509)]
#[feature("safe_dispatcher")]
fn test_cannot_increase_balance_with_zero_value_fork() {
    let contract_address = 0x06731fc32c9970eaea05f4565c0fcf3e8480bc0de9947c905216a3cebfc511b9
        .try_into()
        .unwrap();

    let safe_dispatcher = IHelloStarknetSafeDispatcher { contract_address };

    let balance_before = safe_dispatcher.get_balance().unwrap();
    assert(balance_before == 0, 'Invalid balance');
}
