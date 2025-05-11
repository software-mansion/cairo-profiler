#[cfg(test)]
mod tests {
    use core::{ec::EcPointTrait};
    
    #[test]
    fn range_check_cost() {
        assert(1_u8 >= 1_u8, 'error message');
    }


    #[test]
    fn pedersen_cost() {
        core::pedersen::pedersen(1, 2);
        assert(1 == 1, 'error message');
    }

    #[test]
    fn bitwise_cost() {
        let _bitwise = 1_u8 & 1_u8;
        assert(1 == 1, 'error message');
    }

    #[test]
    fn ec_op_cost() {
        EcPointTrait::new_from_x(1).unwrap().mul(2);
        assert(1 == 1, 'error message');
    }

    #[test]
    fn poseidon_cost() {
        core::poseidon::hades_permutation(0, 0, 0);
        assert(1 == 1, 'error message');
    }
}
