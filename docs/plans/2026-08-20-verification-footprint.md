---
last_updated: 2026-09-05
revision: 1
status: Historical August 20 measurements, extracted from agent workflow during the September 5 context audit; not a current benchmark.
public_safe: true
summary: Dated evidence for disposable build profiles, peak disk accounting, and the two-job CI split.
---

# Verification footprint measurements

On 2026-08-20, both attempts of run 32438837232 died twelve minutes in with
`System.IO.IOException: No space left on device` — thrown by the runner itself,
so **no step log survived at all**. The lane had just gained the clean-clone
proof, which builds this workspace a second time, and nothing anywhere in the
run had ever said what a build cost or how much room it had.

Two defects, and both are fixed rather than worked around: the builds were
larger than they needed to be, and the run could not say so.

**Measured**, in a fresh `CARGO_TARGET_DIR` on rustc 1.96.0, running
`cargo build --workspace --locked`, then `cargo clippy --workspace --locked
--all-targets -- -D warnings`, then `python3 tools/run_rust_tests.py`, with
`du -sm` after each:

| after | default profile | disposable-build profile | no debuginfo at all |
| --- | --- | --- | --- |
| `cargo build` | 3,679 MiB | 1,334 MiB | 951 MiB |
| `cargo clippy --all-targets` | 4,607 MiB | 1,653 MiB | 1,075 MiB |
| the test suite | **21,245 MiB** | **5,910 MiB** | **3,467 MiB** |
| wall clock, all three | 893 s | 689 s | 628 s |

Of the default figure, 9,426 MiB is `debug/incremental` — state whose entire
purpose is making a build nobody will run faster — and most of the rest is
DWARF the test binaries carry. The disposable-build profile
(`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`,
`CARGO_PROFILE_TEST_DEBUG=line-tables-only`) drops the first entirely and keeps
exactly enough of the second to resolve a backtrace to a file and a line. It is
also 23% faster, because writing 15 GB is not free.

The third column is the option that was measured and **not** taken.
`CARGO_PROFILE_*_DEBUG=0` saves a further 2,443 MiB and would let the complete
lane fit one runner — at the price of test failures whose backtraces name no
line. A proof lane exists to say what broke; buying a runner by blinding it is
the wrong trade when a second runner is free.

**It is set in the environment, not in `Cargo.toml`.** The tracked `[profile]`
tables are the *developer's* build, and quietly degrading everybody's debugger
to make a runner fit is paying for CI out of everyone's tooling. The
environment is where a caller declares "this build is disposable", and the two
callers that declare it are the workflow's `env:` block and
`tools/run_clean_clone_proof.py`. A local `--scope full` still builds the way
this tree is configured to build.

**The peak is bigger than the leftovers, and the peak is what has to fit.** The
table above is `du` at rest between steps. `python3 tools/run_clean_clone_proof.py`
samples while it runs, and on 2026-08-20 two runs reported
`TME_CLEAN_CLONE_PEAK_MiB=6831` and `=6792` for the same work that leaves
5,910 MiB behind — about 15% more, because cargo holds superseded artifacts
alongside their replacements before dropping them. A budget computed from the
resting figure is a budget that is 900 MiB wrong in the direction that kills a
job.

**The arithmetic, and why two jobs.** A GitHub-hosted `ubuntu-latest` runner
offers on the order of 14 GB free before the job reclaims anything. One lean
cold build peaks at 6,831 MiB, so a single job running `full` needs two of them
— 13.3 GiB — plus the toolchain, the cargo registry, and the checkout. That is
not a tight fit; it is the same failure again with a smaller number in front of
it. One runner per cold build leaves roughly 7 GiB spare in each, and the two
finish in parallel rather than in series. So CI runs `full` as two jobs, and
the test described above holds them to covering it exactly.

Elapsed, measured on the same day: the clean-clone proof takes 12m 04s
end to end, of which the inner `portable` lane is 12m 01s.

Each job's own log carries the numbers this argument rests on: `df -h /` before
and after the reclamation, the target directory's size after every step that
built, and the clean-clone proof's sampled peak. If the workspace grows past
what this fits, the log says so long before a runner dies of it.

`.github/disk-budget.sh` runs first in both jobs: it prints `df -h /`, removes
the preinstalled payloads this repository has no use for (a .NET SDK, an
Android SDK, a Haskell toolchain, a CodeQL bundle), and prints `df -h /` again.
That is margin, not the fix — the lane is measured to fit without it — and it
is why the reclamation never fails the job.

**And the run says what it is spending.** `--report-disk` prints the target
directory's size and its filesystem's free space after every step that builds,
and the clean-clone proof samples its own build directory while it runs and
prints `TME_CLEAN_CLONE_PEAK_MiB` whether it passed or failed. A run that dies
of a full disk now leaves a log that says how it got there — which is the part
of 2026-08-20 that made the failure expensive.

**GitHub Issues are the working index** for defects and deferred findings. An
issue is where a fix-or-file finding lands. Issue order does not reorder product
priority, and an open issue is not a plan.
