use core::clone::Clone;
use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};

use mega_package::{
    trace_info_checker::{ITraceInfoCheckerDispatcherTrait, ITraceInfoCheckerDispatcher},
    trace_info_proxy::{ITraceInfoProxyDispatcherTrait, ITraceInfoProxyDispatcher},
};

// multiple deploys in snforge;
// there are 11 deploy_syscall usages in trace
// but 14 deploys in nested_calls
#[test]
fn test_call() {
    let empty_hash = declare("Empty").unwrap().contract_class().class_hash.clone();
    let proxy = declare("TraceInfoProxy").unwrap().contract_class();
    let checker = declare("TraceInfoChecker").unwrap().contract_class().clone();
    let dummy = declare("TraceDummy").unwrap().contract_class();

    mega_package::use_builtins_and_syscalls(empty_hash, 7);

    let (checker_address, _) = checker.deploy(@array![]).unwrap();
    let (proxy_address, _) = proxy
        .deploy(@array![checker_address.into(), empty_hash.into(), 5])
        .unwrap();
    let (dummy_address, _) = dummy.deploy(@array![]).unwrap();

    let proxy_dispatcher = ITraceInfoProxyDispatcher { contract_address: proxy_address };
    proxy_dispatcher.regular_call(checker_address, empty_hash, 1);
    proxy_dispatcher.with_libcall(checker.class_hash, empty_hash, 2);
    proxy_dispatcher.call_two(checker_address, dummy_address, empty_hash, 3);

    let chcecker_dispatcher = ITraceInfoCheckerDispatcher { contract_address: checker_address };
    chcecker_dispatcher.from_proxy(4, empty_hash, 4);
}
