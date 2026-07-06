This directory contains compiled files of the sha_hashes contract, along with synthetic traces for testing SHA-512 syscall support.

## Synthetic traces

The `trace_sha_hashes.json`, `trace_sha_hashes_multi_call.json`, `trace_sha_hashes_sierra_gas.json`, and `trace_sha384.json` files were crafted by hand to exercise the `Sha512ProcessBlock` syscall through the `syscall_counter` code path (i.e. without `cairo_execution_info`). They verify that the versioned-constants entry is correct and that the profiler handles `Sha512ProcessBlock` without panicking.

## How to regenerate the real trace

`universal-sierra-compiler >= 2.9.0` is already available (released 2026-06-25). However, snforge VM support for `Sha512ProcessBlock` execution was merged in foundry-rs/starknet-foundry#4459 (2026-07-03) and is not yet in a tagged release. Wait for snforge >= 0.63.0, then run:

```
scarb --version  # should be nightly-2026-05-30 or later
snforge test --save-trace-data
```

Then replace the synthetic traces with the real snforge output (which will include `cairo_execution_info` and exercise the CASM-level code path in `function_trace_builder.rs`).
