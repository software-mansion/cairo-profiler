This directory contains compiled files of other_syscalls contract, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data`, and change paths in trace file to point to correct .json files:
- other_syscalls_integrationtest_SyscallProxy.test.contract_class.json
- other_syscalls_integrationtest.test.sierra.json

Then copy trace + compiled files in here

Generated using:
- scarb 2.19.2
- snforge 0.62.1
