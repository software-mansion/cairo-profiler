This directory contains compiled files of the sha_hashes contract, along with a synthetic trace for testing SHA-512 syscall support.

## trace_sha_hashes.json

This trace was crafted by hand to exercise the `Sha512ProcessBlock` syscall through the `syscall_counter` code path (i.e. without `cairo_execution_info`). It is sufficient to verify that the versioned-constants entry is correct and that the profiler does not panic when it encounters `Sha512ProcessBlock`.

## Sierra artifacts

The Sierra files were compiled with:
- scarb nightly-2026-05-30 (cairo 2.18.0, Sierra 1.8.0)

They are present for future reference and can be used to generate a real execution trace once a compatible version of universal-sierra-compiler (>= 2.9.0) is installed.

## How to regenerate the real trace

Once `universal-sierra-compiler >= 2.9.0` is available, run:

```
scarb --version  # should be nightly-2026-05-30 or later with SHA-512 support
snforge test --save-trace-data
```

Then update `trace_sha_hashes.json` to contain the real snforge trace (pointing to the compiled Sierra files in this directory).

Generated Sierra files using:
- scarb nightly-2026-05-30
- snforge 0.59.0
