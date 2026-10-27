# Benchmarks

Three micro-benchmarks, three different questions. Common protocol:
warmup of ALL the variants before measuring, then INTERLEAVED rounds
(round-robin, never in parallel) with the order rotated every round, and
the median of the results. This way an external load that comes and goes
hits all the variants the same way, instead of contaminating one block.

| Script | Question | Metric |
|---|---|---|
| `run-bench.ps1` | how fast is the walk? | internal ms (a stopwatch inside the program, runtime startup excluded) |
| `startup-bench.ps1` | how much does starting cost? | external ms of the whole process; startup estimated by difference |
| `mem-bench.ps1` | how much RAM does it use? | working set and commit peaks (kernel counters, read via NtQueryInformationProcess) |

## How to run them

```powershell
# prerequisites
cd rust ; cargo build --release ; cd ..
cd net  ; dotnet build -c Release ; cd ..
# the C# R2R / self-contained variants need the publish described at
# the top of the scripts: without it, the scripts skip them with a
# warning

powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target C:\Windows -Runs 5
powershell -ExecutionPolicy Bypass -File bench\startup-bench.ps1 -Runs 7
powershell -ExecutionPolicy Bypass -File bench\mem-bench.ps1 -Target C:\Windows -Runs 5
```

## The RESULT contract

With `MFD_BENCH=1` the walkers emit only machine-readable lines:

```
RESULT ms=4 bytes=123456789 files=171000 dirs=12000 errors=0
ITER 3/5 ms=4 ...
```

`run-bench.ps1` fails if the RESULT line is missing and, at the end of
the run, verifies that the totals (bytes/files/dirs) match across all
the implementations: the cross-check that the walker does the same thing
in every language.

## Method notes

- The external time measured by PowerShell includes a constant ~3-5 ms
  of process spawn: visible in the rust total, identical for all the
  variants.
- Node runs with `UV_THREADPOOL_SIZE=16` (the default 4 would penalize
  it): Node gets its best configuration.
- `startup-bench.ps1` uses the repo itself as the default target (a
  small folder: the external time is dominated by startup, which is the
  quantity under study).

The results of the measurement sessions live in `bench\results\`.

What the three benches decided — the scorecard, the crowns and who wins
what — is told in the READMEs (The experiment).
