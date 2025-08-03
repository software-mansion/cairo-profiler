This directory contains compiled files of tree_verification code, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data --tracked-resource sierra-gas`, and change paths in trace files to point to correct .json files:
- mega_package_integrationtest.test.sierra.json
- mega_package_integrationtest_ERC20.test.contract_class.json
- mega_package_integrationtest_TraceInfoChecker.test.contract_class.json
- mega_package_integrationtest_TraceInfoProxy.test.contract_class.json
- mega_package_integrationtest_TraceDummy.test.contract_class.json

Then copy trace + compiled files in here

Generated using:
- scarb 2.11.4
- snforge 0.48.0


There are additional trace files here that are based off of `mega_package_integrationtest_test_calls_test_call.json`; these are prepared to test specific test scenarios:
- mega_package_more_triggers_than_calls.json
    - removed last `EntryPointCall` from nested_calls of top level entrypoint `SNFORGE_TEST_CODE`
- mega_package_more_calls_than_triggers.json
    - added another `EntryPointCall` to nested_calls of top level entrypoint `SNFORGE_TEST_CODE`
- mega_package_mismatched.json
    - changed last `EntryPointCall`s `call_type` field from `Call` to `Delegate` in nested_calls of top level entrypoint `SNFORGE_TEST_CODE`
