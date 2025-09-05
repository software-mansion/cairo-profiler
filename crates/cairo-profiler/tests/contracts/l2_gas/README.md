This directory contains compiled files of l2_verification code, along with its trace data (to simplify testing)
To re-generate run `snforge test --save-trace-data --tracked-resource sierra-gas`, and change paths in trace files to point to correct .json files:
- l2_verification_integrationtest.test.sierra.json
- l2_verification_integrationtest_ERC20.test.contract_class.json

Then copy trace + compiled files to `precompiled_sierra_gas` directory, and repeat the process for cairo steps - this time running `snforge test --save-trace-data --tracked-resource cairo-steps` and copying resulting files (with changed paths for artifacts!) to `precompiled_cairo_steps` fdirectory.

Generated using:
- scarb 2.12.0
- snforge 0.49.0
