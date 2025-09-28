
#[executable]
fn with_syscalls() {
    let mut input = array![0x6f77206f6c6c6548];
    core::keccak::cairo_keccak(ref input, 0x21646c72, 4);

    1_u8 >= 1_u8;
    1_u8 & 1_u8;

    core::pedersen::pedersen(1, 2);
    core::poseidon::hades_permutation(0, 0, 0);

    let ec_point = core::ec::EcPointTrait::new_from_x(1).unwrap();
    core::ec::EcPointTrait::mul(ec_point, 2);

    core::keccak::keccak_u256s_le_inputs([1].span());
}

#[executable]
fn dummy() -> felt252 {
    println!("hello");
    0
}

#[executable]
fn with_arguments(first: u8, second: felt252) -> felt252 {
    let sum: felt252 = first.into() + second;

    println!("{}", sum);
    sum
}
