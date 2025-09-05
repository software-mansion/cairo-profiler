use l2_verification::erc20::{IERC20Dispatcher, IERC20DispatcherTrait};
use snforge_std::cheatcodes::contract_class::DeclareResultTrait;
use snforge_std::{
    ContractClassTrait, declare, start_cheat_caller_address, stop_cheat_caller_address,
    test_address, start_cheat_signature_global, stop_cheat_signature_global
};
use starknet::ContractAddress;

const NAME: felt252 = 'TOKEN';
const SYMBOL: felt252 = 'TKN';
const DECIMALS: u8 = 2;
const INITIAL_SUPPLY: u256 = 10;

fn deploy_erc20(
    name: felt252, symbol: felt252, decimals: u8, initial_supply: u256, recipient: ContractAddress,
) -> ContractAddress {
    let contract = declare("ERC20").unwrap().contract_class();

    let mut constructor_calldata: Array<felt252> = array![name, symbol, decimals.into()];

    let mut initial_supply_serialized = array![];
    initial_supply.serialize(ref initial_supply_serialized);

    constructor_calldata.append_span(initial_supply_serialized.span());
    constructor_calldata.append(recipient.into());

    let (address, _) = contract.deploy(@constructor_calldata).unwrap();
    address
}

#[test]
fn with_signature() {
    start_cheat_signature_global(array![1234].span());

    let erc20_address = deploy_erc20(NAME, SYMBOL, DECIMALS, INITIAL_SUPPLY, test_address());
    let dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    let spender: ContractAddress = 123.try_into().unwrap();
    dispatcher.transfer(spender, 2.into());

    let spender_balance = dispatcher.balance_of(spender);
    assert(spender_balance == 2, 'invalid spender balance');

    start_cheat_caller_address(erc20_address, spender);

    dispatcher.increase_allowance(test_address(), 2);

    let allowance = dispatcher.allowance(spender, test_address());
    assert(allowance == 2, 'invalid allowance');

    stop_cheat_caller_address(erc20_address);

    dispatcher.transfer_from(spender, test_address(), 2);

    let allowance = dispatcher.allowance(spender, test_address());
    assert(allowance == 0, 'invalid allowance');

    let spender_balance = dispatcher.balance_of(spender);
    assert(spender_balance == 0, 'invalid spender balance');

    let balance = dispatcher.balance_of(test_address());
    assert(balance == INITIAL_SUPPLY, 'invalid balance');

    stop_cheat_signature_global();
}

#[test]
fn without_signature() {
    let erc20_address = deploy_erc20(NAME, SYMBOL, DECIMALS, INITIAL_SUPPLY, test_address());
    let dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    let spender: ContractAddress = 123.try_into().unwrap();
    dispatcher.transfer(spender, 2.into());

    let spender_balance = dispatcher.balance_of(spender);
    assert(spender_balance == 2, 'invalid spender balance');

    start_cheat_caller_address(erc20_address, spender);

    dispatcher.increase_allowance(test_address(), 2);

    let allowance = dispatcher.allowance(spender, test_address());
    assert(allowance == 2, 'invalid allowance');

    stop_cheat_caller_address(erc20_address);

    dispatcher.transfer_from(spender, test_address(), 2);

    let allowance = dispatcher.allowance(spender, test_address());
    assert(allowance == 0, 'invalid allowance');

    let spender_balance = dispatcher.balance_of(spender);
    assert(spender_balance == 0, 'invalid spender balance');

    let balance = dispatcher.balance_of(test_address());
    assert(balance == INITIAL_SUPPLY, 'invalid balance');
}
