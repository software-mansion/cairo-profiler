#!/bin/bash

cairo-profiler "./trace.json"
go tool pprof -http=":8000" profile.pb.gz
