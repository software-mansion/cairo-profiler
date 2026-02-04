#[starknet::interface]
pub trait ISyscallProxy<TContractState> {
    fn other_syscalls(self: @TContractState);
}

#[starknet::contract]
pub mod SyscallProxy {
    use starknet::SyscallResultTrait;
    use core::keccak::keccak_u256s_le_inputs;
    use starknet::secp256_trait::{Secp256PointTrait, Secp256Trait, Signature, recover_public_key};
    use starknet::secp256k1::{Secp256k1Point};
    use starknet::secp256r1::{Secp256r1Point};

    #[storage]
    struct Storage {}

    #[abi(embed_v0)]
    impl Secp256r1BenchImpl of super::ISyscallProxy<ContractState> {
        fn other_syscalls(self: @ContractState) {
            let msg_hash = 0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855;
            let r = 0xb292a619339f6e567a305c951c0dcbcc42d16e47f219f9e98e76e09d8770b34a;
            let s = 0x177e60492c5a8242f76f07bfe3661bde59ec2a17ce5bd2dab2abebdf89a62e2;
            let public_key = recover_public_key::<Secp256r1Point>(
                msg_hash, Signature { r, s, y_parity: true }
            ).unwrap();
            public_key.get_coordinates().unwrap_syscall();

            let k1_generator = Secp256Trait::<Secp256k1Point>::get_generator_point();
            k1_generator.get_coordinates().unwrap_syscall();
            k1_generator.add(k1_generator).unwrap_syscall();
            k1_generator.mul(2).unwrap_syscall();

            let k1_point = Secp256Trait::<Secp256k1Point>::secp256_ec_get_point_from_x_syscall(
                0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798,
                false,
            )
                .unwrap_syscall()
                .unwrap();
            k1_point.get_coordinates().unwrap_syscall();

            let _ = keccak_u256s_le_inputs(array![0_u256, 1_u256].span());
        }
    }
}
