This directory contains compiled files of builtins_simple code, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data --tracked-resource sierra-steps`, and change paths in trace files to point to correct .json files:
- deploy_syscall_simple_unittest.test.sierra.json
- deploy_syscall_simple_unittest_GasConstructorChecker.test.contract_class.json

Then copy trace + compiled files in here

Generated using:
- scarb 2.11.3
- snforge 0.39.0

