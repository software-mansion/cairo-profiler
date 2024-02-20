# cairo-profiler

Profiler for Cairo programming language &amp; Starknet.

## Installation

To install the latest version of cairo-profiler, run:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/cairo-profiler/main/scripts/install.sh | sh
```

If you want to install a specific a version, run the following command with the requested version:

```shell
curl -L https://raw.githubusercontent.com/software-mansion/cairo-profiler/main/scripts/install.sh | sh -s -- v0.1.0
```

## External tools integration

`cairo-profiler` is a tool-agnostic profiler which means that it accepts input from any tool. Those tools need to generate
trace in the [expected](https://github.com/software-mansion/cairo-profiler/blob/main/src/trace_data.rs) format.

### Integrated tools

- [x] [Starknet Foundry](https://github.com/foundry-rs/starknet-foundry) - check how to generate the input file [here](https://foundry-rs.github.io/starknet-foundry/testing/profiling.html)

## Usage

Usage flow consists of two steps:

- generating the output file,
- running `pprof` tool.

### Generating output file

To generate the file run `cairo-profiler` with the `<PATH_TO_TRACE_DATA>` argument containing
the json file path with the trace to be profiled. You can also specify the `--output-path <OUTPUT_PATH>` -
by default the output file will be saved as `profile.pb.gz`

#### Example

```shell
cairo-profiler path/to/trace.json
```

> 📝 **Note**
>
> Trace needs to be in the correct format. See [trace.json](./tests/data/trace.json) as an example.

### Running pprof

To see results from the generated file you will need to install:

- [go](https://go.dev/doc/install)
- [graphviz](https://www.graphviz.org/download/)
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
  - [ ] L1 resources profiling - costs of storage on L1: contract updates, L2 -> L1 messages
- [ ] Function level profiling
- [ ] Exposing `cairo-profiler` library to allow other tools to integrate

## Setup for development

You need to install: [Rust](https://www.rust-lang.org/tools/install), [Go](https://go.dev/doc/install), 
[protoc](https://grpc.io/docs/protoc-installation), [pprof](https://github.com/google/pprof?tab=readme-ov-file#building-pprof) and [graphviz](https://graphviz.org/download). 
