This directory contains compiled sierra files of multiple_targets code, along with its trace data (to simplify testing).
To re-generate run:
- `scarb execute --save-profiler-trace-data --executable-name="with_syscalls"`
- `scarb execute --save-profiler-trace-data --executable-name="with_arguments" --arguments 1,2`
- `scarb execute --save-profiler-trace-data --executable-name="dummy"` 


and change paths in respective trace files to point to correct .json files:
- with_syscalls.executable.sierra.json
- with_arguments.executable.sierra.json
- dummy.executable.sierra.json

Then copy trace + compiled files in here and repeat with `scarb execute (...) --target="bootloader"` (remember to copy its trace file and change paths as well).

Generated using:
- scarb 2.13.1
