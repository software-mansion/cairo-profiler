This directory contains compiled files of builtins_simple code, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data --tracked-resource sierra-gas`, and change paths in trace files to point to correct .json files:
- scaled_syscall_unittest.test.sierra.json
- scaled_syscall_unittest_GasConstructorChecker.test.contract_class.json

Then copy trace + compiled files in here

Generated using:
- scarb 2.11.3
- snforge 0.43.1
