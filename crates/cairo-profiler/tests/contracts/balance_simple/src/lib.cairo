#[starknet::interface]
pub trait IHelloStarknet<TContractState> {
    fn get_balance(self: @TContractState) -> felt252;
}

#[starknet::contract]
mod HelloStarknet {
    use core::starknet::storage::StoragePointerReadAccess;

    #[storage]
    struct Storage {
        balance: felt252,
    }

    #[abi(embed_v0)]
    impl HelloStarknetImpl of super::IHelloStarknet<ContractState> {
        fn get_balance(self: @ContractState) -> felt252 {
            self.balance.read()
        }
    }
}
