# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.0] - 2025.10.01

### Added

- support for scarb execute

## [0.13.0] - 2025.09.24

### Fixed

- sierra gas estimations for syscalls were inaccurate, because of a bug in builtins counting logic 

## [0.12.0] - 2025.09.05

### Added

- l2 gas profiling

## [0.11.0] - 2025.08.07

### Fixed

- profiler now correctly removes `::` separators from function names which were monomorphised with different types
- calldata factor for deploy syscall is now correctly factored in

### Changed

- upgraded cost map (versioned constants) to starknet 0.14.1
- contract entrypoints are now shown in the tree as called from functions

## [0.10.0] - 2025-07-18

### Changed

- addresses and selectors are now displayed in fixed hex format (66 chars) instead of decimal format

### Fixed

- profiler now accounts for steps when pc is outside the function area
- profiler now accounts for syscalls in nodes without `CairoExecutionInfo`

## [0.9.0] - 2025-05-20

### Added

- sierra gas profiling; in order to profile sierra gas please make sure to run snforge test with `--tracked-resource` flag set to "sierra-gas"
- builtins usage is now included in sierra gas estimations
- new flag `--show-libfuncs`, allowing to show all libfuncs usage per function (along with its resource consumption)
- new sample "casm size" to show casm sizes of functions
- new sample "syscall usage" to show functions' syscall usage count

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
