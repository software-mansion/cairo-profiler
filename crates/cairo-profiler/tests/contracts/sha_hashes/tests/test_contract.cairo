use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use sha_hashes::{IShaHashesDispatcher, IShaHashesDispatcherTrait};

#[test]
fn sha512_hash_test() {
    let contract = declare("ShaHashes").unwrap().contract_class();
    let (addr, _) = contract.deploy(@array![]).unwrap();
    let dispatcher = IShaHashesDispatcher { contract_address: addr };
    dispatcher.sha512_hash();
}
