use snforge_std::cheatcodes::contract_class::DeclareResultTrait;
use starknet::ContractAddress;

use snforge_std::{
    declare, ContractClassTrait, test_address,
};

use mega_package::erc20::IERC20Dispatcher;
use mega_package::erc20::IERC20DispatcherTrait;

const NAME: felt252 = 'TOKEN';
const SYMBOL: felt252 = 'TKN';
const DECIMALS: u8 = 2;
const INITIAL_SUPPLY: u256 = 10;

fn deploy_erc20(
    name: felt252, symbol: felt252, decimals: u8, initial_supply: u256, recipient: ContractAddress
) -> ContractAddress {
    let contract = declare("ERC20").unwrap().contract_class();

    let mut constructor_calldata: Array::<felt252> = array![name, symbol, decimals.into()];

    let mut initial_supply_serialized = array![];
    initial_supply.serialize(ref initial_supply_serialized);

    constructor_calldata.append_span(initial_supply_serialized.span());
    constructor_calldata.append(recipient.into());

    let (address, _) = contract.deploy(@constructor_calldata).unwrap();
    address
}

// no deploy syscall in trace
#[test]
fn test() {
    let erc20_address = deploy_erc20(NAME, SYMBOL, DECIMALS, INITIAL_SUPPLY, test_address());
    let dispatcher = IERC20Dispatcher { contract_address: erc20_address };

    let spender: ContractAddress = 123.try_into().unwrap();
    dispatcher.transfer(spender, 2.into());

}
