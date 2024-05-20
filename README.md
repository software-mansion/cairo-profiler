# cairo-profiler

Profiler for Cairo programming language &amp; Starknet.

![Example output](.github/images/demo.gif)

## Installation

To install the latest stable version of `cairo-profiler`, run:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/cairo-profiler/main/scripts/install.sh | sh
```

If you want to install a specific a version, run the following command with the requested version:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/cairo-profiler/main/scripts/install.sh | sh -s -- v0.1.0
```

### Installation on Windows

As for now, `cairo-profiler` on Windows needs manual installation, but necessary steps are kept to minimum:

1. [Download the release](https://github.com/software-mansion/cairo-profiler/releases) archive matching your CPU architecture.
2. Extract it to a location where you would like to have `cairo-profiler` installed. A folder named cairo_profiler in your [`%LOCALAPPDATA%\Programs`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid?redirectedfrom=MSDN#FOLDERID_UserProgramFiles) directory will suffice:
```batch
%LOCALAPPDATA%\Programs\cairo_profiler
```
3. Add path to the cairo_profiler\bin directory to your PATH environment variable.
4. Verify installation by running the following command in new terminal session:
```shell
cairo-profiler --version
```

## External tools integration

`cairo-profiler` is a tool-agnostic profiler which means that it accepts input from any tool. Those tools need to generate
trace in the [expected](./crates/trace-data/src/lib.rs) format.

### Integrated tools

- [x] [Starknet Foundry](https://github.com/foundry-rs/starknet-foundry) - check how to generate the input file [here](https://foundry-rs.github.io/starknet-foundry/testing/profiling.html)

## Usage

Usage flow consists of two steps:

- generating the output file,
- running `pprof` tool.

### Generating output file

To generate the file run `cairo-profiler` with the `<PATH_TO_TRACE_DATA>` argument containing
the path to the json file with the trace to be profiled. You can also specify the `--output-path <OUTPUT_PATH>` -
if not specified, the output file will be saved as `profile.pb.gz`.

#### Example

```shell
cairo-profiler path/to/trace.json
```

> 📝 **Note**
>
> Trace needs to be in the correct format. See [trace.json](./crates/cairo-profiler/tests/data/call.json) as an example.

### Running pprof

To see results from the generated file you will need to install:

- [Go](https://go.dev/doc/install)
- [Graphviz](https://www.graphviz.org/download/)
- [pprof](https://github.com/google/pprof?tab=readme-ov-file#building-pprof)

and run:

```shell
go tool pprof -http=":8000" profile.pb.gz
```

This command will start a web server at the specified port that provides an interactive interface.
You can learn more about pprof usage options [here](https://github.com/google/pprof?tab=readme-ov-file#basic-usage).

## Roadmap

`cairo-profiler` is under active development! Expect a lot of new features to appear soon! 🔥

- [ ] Starknet calls profiling:
  - [x] L2 resources - steps, memory holes, builtins, syscalls 
  - [ ] L1 resources - contract updates, L2 -> L1 messages
- [ ] Function level profiling:
  - [x] Steps profiling
  - [ ] Builtins profiling
  - [ ] Memory holes profiling
  - [ ] Information about inlined functions
- [ ] Integrating with other tools:
  - [x] Exposing `cairo-profiler` library to allow other tools to integrate
  - [x] Integrating with [`snforge`](https://github.com/foundry-rs/starknet-foundry)
  - [ ] Integrating with `cairo-test` and `cairo-run`

## Development

### Environment setup
You need to install: [Rust](https://www.rust-lang.org/tools/install), [Go](https://go.dev/doc/install), 
[protoc](https://grpc.io/docs/protoc-installation), [pprof](https://github.com/google/pprof?tab=readme-ov-file#building-pprof) and [Graphviz](https://graphviz.org/download). 

### Running the binary

The binary can be run with:

```shell
cargo run <PATH_TO_TRACE_DATA>
```

### Running tests

Tests can be run with:

```shell
cargo test
```

### Formatting and lints

`cairo-profiler` uses [rustfmt](https://github.com/rust-lang/rustfmt) for formatting. You can run the formatter with:

```shell
cargo fmt
```

For linting, it uses [clippy](https://github.com/rust-lang/rust-clippy). You can run it using our defined alias:

```shell
cargo lint
```

### Spelling

`cairo-profiler` uses [typos](https://github.com/marketplace/actions/typos-action) for spelling checks.

You can run the checker with

```shell
typos
```

Some typos can be automatically fixed by running

```shell
typos -w
```
