# cairo-profiler
Profiler for cairo programming language &amp; starknet

## External tools integration

`cairo-profiler` is tool-agnostic which means that it accepts input from any tool. Those tools need to generate
trace in the [expected](https://github.com/software-mansion/cairo-profiler/blob/d5a5a2722fdf81a0cbabfa44a91cbd69ebe7110d/src/trace_data.rs#L10) format.

### Integrated tools

- [x] [Starknet Foundry](https://foundry-rs.github.io/starknet-foundry/index.html)

## Installation

To start using `cairo-profiler` download archive file from the GitHub releases

```shell
wget https://github.com/software-mansion/cairo-profiler/releases/download/v0.1.0/cairo-profiler-v0.1.0-aarch64-apple-darwin.tar.gz
```

and unpack it.

> 📝 **Note**
>
> Make sure to download the correct file (suitable for your machine).

## Usage

Usage flow consists of two steps:

- first we need to generate the `profile.pb.gz` file,
- and then run `pprof` tool.

### Generate pprof file

To generate the file run `cairo-profiler` with the `<PATH_TO_TRACE_DATA>` argument.
It is the json file containing the trace to be profiled.

#### Example

```shell
cairo-profiler path/to/trace.json
```

> 📝 **Note**
>
> Trace needs to be in the correct format. See [trace.json](./tests/data/trace.json) file.

### See profiler graph

To see results from the generated file you will need to install:

- [go](https://go.dev/doc/install)
- [graphviz](https://www.graphviz.org/download/)
- [pprof](https://github.com/google/pprof?tab=readme-ov-file#building-pprof)

and run:

```shell
go tool pprof -http=":8000" profile.pb.gz
```

This command will start a web server at the specified port that provides an interactive interface.
More info [here](https://github.com/google/pprof?tab=readme-ov-file#run-pprof-via-a-web-interface).

## Roadmap

`cairo-profiler` is under active development! Expect a lot of new features to appear soon! 🔥

- [x] Starknet calls profiling
- [ ] Pure Cairo profiling
- [ ] Exposing `cairo-profiler` library allowing other tools to integrate
- [ ] L1 resources (storage cost, ...)

## Setup for development

You need to install: rust, go, protobuf, pprof, graphviz 

### Building protobufs

Protocol buffers are built with [prost-build](https://github.com/tokio-rs/prost/tree/master/prost-build)

### Run profiling

```
cargo run
```

### Reading the profiling results

```
go tool pprof -http=":8000" profile.pb.gz
```
