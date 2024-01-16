# cairo-profiler
Profiler for cairo programming language &amp; starknet

## Setup for development

You need to install: rust, go, protobuf, pprof, graphviz 

## Building protobufs
Protocol buffers are built with [prost-build](https://github.com/tokio-rs/prost/tree/master/prost-build)

## Run profiling
Run the command from the profiler directory
```
cargo run
```

## Reading the profiling results
Run the command from the profiler directory
```
go tool pprof -http=":8000" profile.pb.gz
```
