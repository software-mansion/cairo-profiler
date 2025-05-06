# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.2] - 2025-05-06

### Added

- new `--hide` option, that allows to filter out nodes from the profile based on passed regex. Resources of the filtered
  node are added to a parent node (function).

## [0.8.1] - 2025-02-25

### Chore
- Bumped dependencies to ensure compatibility with scarb 2.10 and newest snfoundry

## [0.8.0] - 2025-01-13

### Fixed

- `l2_l1_message_sizes` sample no longer is displayed when its value is zero

### Added

- new `build-profile` subcommand, it behaves the same as `cairo-profiler <trace_file>`
- new flag `--view` that allows to view the built profile 
(eg `cairo-profiler <trace_file> --view` or `cairo-profiler build-profile <trace_file> --view`)
- new `view` subcommand that allows to view previously built profile

## [0.7.0] - 2024-11-27

- show syscalls as nodes in trace tree rather than samples
- add `--versioned-constants-path` flag to allow passing custom resource cost map
- remove `trace-data` crate in favour of `cairo-annotations`

## [0.6.0] - 2024-08-22

- support for `sha256_process_block_syscall`

## [0.5.0] - 2024-08-01

### Added

- support for Sierra 1.6.0

## [0.5.0-rc.0] - 2024-07-03

### Added

- `--show-inlined-functions` flag to show inlined functions in the profile. Requires Scarb >= 2.7.0-rc.0 and setting
  `unstable-add-statements-functions-debug-info = true` in `[cairo]` section of Scarb.toml.

## [0.4.0] - 2024-06-12

### Added

- `--max-function-stack-trace-depth` allowing to specify maximum depth of the function tree in function level profiling
- `--split-generics` flag allowing to differentiate between non-inlined generics monomorphised with different types
 
## [0.3.0] - 2024-05-20

### Added

- function level profiling for steps

## [0.3.0-dev.0] - 2024-04-17

### Added

- optional field and `CallEntryPoint.class_hash` to input structs
- optional field `CallEntryPoint.cairo_execution_info` to input structs. The struct contains vm trace and path to a
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
