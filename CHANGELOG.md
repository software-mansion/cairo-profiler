# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- optional field and `CallEntryPoint.class_hash` to input structs
- optional field `CallEntryPoint.function_level_info` to input structs. The struct contains vm trace and path to a
  relevant sierra file. It will enable function level profiling soon

### Changed

- `CallTrace.nested_calls` type changed from `Vec<CallTrace>` to `Vec<CallTraceNode>`

## [0.2.0] - 2024-03-08

### Added

- `trace_data` library (other tools can use it to integrate with `cairo-profiler`)
- L2 -> L1 messages
- `show_details` flag to show `contract_address` and `function_selector`

### Changed

- `contract_address` and `function_selector` are not displayed by default (use `show_details` flag to see them)

## [0.1.0] - 2024-02-21

#### Added

- Starknet calls profiling:
    - L2 resources: steps, memory holes, builtins, syscalls
    - support for human-readable contract and function names
- custom output path support
