#!/usr/bin/env bash
# test.sh — run the test suite so that a runaway test dies ALONE.
#
# 🔴 Why this exists, measured 2026-09-01 (`B-030`, `FIND-218`): `tests/vector_hooks.rs`
# ordered a system in `SimulationSystems::World` `.after(aim)` while `aim` had moved to
# `SimulationSystems::PostStep`. The six stages are `.chain()`ed, so that closed a dependency
# cycle in `FixedUpdate` — and bevy answers a cycle by enumerating EVERY simple cycle in the
# component and formatting all of them into one `String`
# (`bevy_ecs-0.19.0/src/schedule/error.rs:174`). Here: **2 290 028 cycles**, a single `realloc`
# of **4 966 055 936 bytes**, `total-vm 81 GB / anon-rss 25 GB`, and the kernel's OOM killer
# took the user's whole tmux session down with the test binary.
#
# The one-line mistake is fixed. The SHAPE is not: any future cycle in any schedule does the
# same thing, and the number is combinatorial in the size of the strongly connected component,
# not in anything a reviewer can eyeball. So the suite runs under a hard per-process address
# limit. A runaway then hits `memory allocation of N bytes failed` and dies alone.
#
#   tools/test.sh                       # --lib plus every integration binary
#   tools/test.sh --lib --test world    # the round-gate cut: pass cargo's own args
#   DBT_TEST_CAP_KB=8388608 tools/test.sh   # a bigger cap, if a test really needs it
#
# 🔴 AND THE ORDER OF THE TWO HALVES IS THE POINT — do not "simplify" it back into one
# command. `ulimit -v` applies to every child, and **`mold` reserves 8 GB of virtual memory
# up front**: under a 6 GB cap the LINKER dies, not the test, with
# `mold: cannot reserve 8589934592 bytes of virtual memory` — a link error wearing a memory
# error's clothes. Measured the same morning, on the very invocation that was supposed to be
# the safe one. So: **compile uncapped, run capped.**
set -u
cd "$(dirname "$0")/.." || exit 1

CAP_KB="${DBT_TEST_CAP_KB:-6291456}"          # 6 GiB of address space per test process
ARGS=("$@")
[ ${#ARGS[@]} -gt 0 ] || ARGS=()

echo "== compiling (uncapped: mold needs 8 GB of VM to link) =="
log=$(mktemp -t dbt-test-XXXXXX.log)
nice -n 15 ionice -c 3 cargo test --no-run -j 3 "${ARGS[@]}" >"$log" 2>&1
rc=$?
grep -E '^error' "$log" | head -20
if [ $rc -ne 0 ]; then
  echo "COMPILE FAILED (exit $rc) — nothing was run; full log: $log"
  exit $rc
fi

# 🔴 ONE run, not two. `cargo test` is 181 s at best here, so re-running it just to read `$?`
# is the exact waste `CLAUDE.md` bans. The status comes out of `${PIPESTATUS[0]}` — the FIRST
# command of the pipeline — because a pipeline's own `$?` is the LAST command's and `grep`
# succeeds whenever it matched anything at all.
echo "== running (ulimit -v ${CAP_KB} KB per process) =="
# ⚠️ NO DBT_SAVE_DIR here, and that is a decision with a test behind it: a test binary already
# refuses a save directory on its own (`save::file::in_a_test_binary` detects `target/*/deps`),
# and `a_test_binary_does_not_get_a_save_directory` PINS that. Exporting a scratch dir from here
# overrode the refusal and turned that guard's assertion vacuous — measured 2026-09-01, it went
# red the first run. The processes that really do write the player's save are SCRIPT runs of the
# real binary, and `tools/corpus.sh` isolates those per run (B-036).
( ulimit -v "$CAP_KB"; nice -n 15 ionice -c 3 cargo test -j 3 "${ARGS[@]}" 2>&1 ) | tee "$log" \
  | grep -E '^test result|^error|panicked|FAILED|memory allocation|does not build'
rc=${PIPESTATUS[0]}
echo "== exit $rc  (full log: $log) =="
exit $rc
