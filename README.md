# cairo-profiler
Profiler for cairo programming language &amp; starknet

## Setup

You need to install: rust, go, protobuf, pprof, 

## Building protobufs
Protocol buffers are built with [prost-build](https://github.com/tokio-rs/prost/tree/master/prost-build)

## Run profiling
Run the command
```
cargo run
```

## Reading the profiling results
Run the command
```
go tool pprof -http=":8000" profile.pb.gz
```
