This directory contains compiled files of balance_simple contract, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data --tracked-resource sierra-gas`, and change paths in trace file to point to correct .json files:
- balance_simple_integrationtest_HelloStarknet.test.contract_class.json
- balance_simple_integrationtest.test.sierra.json

Then copy trace + compiled files in here

Generated using:
- scarb 2.11.4
- snforge 0.46.0
