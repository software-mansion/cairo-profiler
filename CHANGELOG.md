# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-03-08

### Added

- option to pass path to the output path
- `trace_data` library (other tools can use it to integrate with `cairo-profiler`)
- L1 -> L2 messages
- `show_details` flag to show `contract_address` and `function_selector`

### Changed

- `contract_address` and `function_selector` are not displayed by default (use `show_details` flag to see them)

## [0.1.0] - 2024-02-21

#### Added

- Starknet calls profiling:
  - L2 resources: steps, memory holes, builtins, syscalls
  - support for human-readable contract and function names
- custom output path support
