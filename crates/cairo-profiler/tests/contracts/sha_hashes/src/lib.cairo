#[starknet::interface]
pub trait IShaHashes<TContractState> {
    fn sha512_hash(self: @TContractState);
    fn sha384_hash(self: @TContractState);
}

#[starknet::contract]
pub mod ShaHashes {
    use core::sha512::compute_sha512_byte_array;
    use core::sha384::compute_sha384_byte_array;

    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    impl ShaHashesImpl of super::IShaHashes<ContractState> {
        fn sha512_hash(self: @ContractState) {
            let data = "Hello world";
            let _hash = compute_sha512_byte_array(@data);
        }

        fn sha384_hash(self: @ContractState) {
            let data = "Hello world";
            let _hash = compute_sha384_byte_array(@data);
        }
    }
}
