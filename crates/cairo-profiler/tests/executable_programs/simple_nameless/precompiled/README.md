This directory contains compiled sierra file of simple_nameless code, along with its trace data (to simplify testing).
To re-generate run `scarb execute --save-profiler-trace-data` , and change paths in trace files to point to correct .json files:
- simple_nameless.executable.sierra.json

Then copy trace + compiled files in here and repeat with `scarb execute --save-profiler-trace-data --target="bootloader"` (remember to copy its trace file and change paths as well).

Generated using:
- scarb 2.13.1
