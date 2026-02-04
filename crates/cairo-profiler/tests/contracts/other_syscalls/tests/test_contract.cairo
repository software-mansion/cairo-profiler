use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};
use other_syscalls::{ISyscallProxyDispatcher, ISyscallProxyDispatcherTrait};


#[test]
fn other_syscalls_test() {
    let contract = declare("SyscallProxy").unwrap().contract_class();
    let (addr, _) = contract.deploy(@array![]).unwrap();
    let dispatcher = ISyscallProxyDispatcher { contract_address: addr };
    dispatcher.other_syscalls();
}