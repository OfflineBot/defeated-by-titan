#!/usr/bin/env bash
# corpus.sh — run every evidence script AT ITS OWN documented invocation, in parallel.
#
# Why this exists: the supervisor hand-rolled this sweep twice on 2026-08-29 and got it wrong
# both times, reporting 43 scripts red where the truth was 30 — purely from reading the FIRST
# `--ticks` out of a header instead of the LARGEST, so twenty healthy scripts were cut off
# mid-run and printed "did not finish". A run that is cut off has not failed; it has not
# finished, and the two look nothing alike once you stop guessing.
#
#   tools/corpus.sh              every script, one line each
#   tools/corpus.sh red          only the ones that actually failed an assert
#   tools/corpus.sh red f0       only failures whose name matches f0
#
# It never edits anything and it never takes an exit code from a pipeline.
set -u
cd "$(dirname "$0")/.." || exit 1
BIN=target/debug/defeated_by_titan
[ -x "$BIN" ] || { echo "build it first: cargo build"; exit 1; }
JOBS="${CORPUS_JOBS:-6}"

one() {
  f="$1"; b=$(basename "$f" .txt)
  # 🔴 FLAGS AND TICKS COME FROM THE DOCUMENTED INVOCATION LINE ONLY — the line that names
  # `--headless` AND this very script. Reading the whole file was this tool's own first bug: a
  # comment saying "launches without `--hub`" (p1-overlay.txt:59) made it pass `--hub`, and
  # "**No `--mission`.**" (f-flight-cut.txt:32) made it pass `--mission tutorial`. Three healthy
  # scripts were reported RED for it, by a tool whose header is about not guessing invocations.
  inv=$(grep -E -- '--headless' "$f" | grep -F -- "$(basename "$f")" | head -3)
  [ -n "$inv" ] || inv=$(grep -E -- '--headless' "$f" | head -3)
  t=$(printf '%s\n' "$inv" | grep -oE -- '--ticks [0-9]+' | awk '{print $2}' | sort -rn | head -1)
  [ -n "$t" ] || t=$(grep -oE -- '--ticks [0-9]+' "$f" | awk '{print $2}' | sort -rn | head -1)
  [ -n "$t" ] || t=1500
  x=""
  printf '%s\n' "$inv" | grep -qE -- '--hub' && x="$x --hub"
  m=$(printf '%s\n' "$inv" | grep -oE -- '--mission [a-z]+' | head -1)
  [ -n "$m" ] && x="$x $m"
  out=$(timeout 180 nice -n 15 ionice -c 3 ./target/debug/defeated_by_titan \
        --headless $x --script "$f" --ticks "$t" 2>&1 \
        | grep -oE '[0-9]+ of [0-9]+ asserts failed|[0-9]+ asserts held|did not finish' | tail -1)
  case "$out" in
    *"asserts held")  printf "GREEN  %-24s %-6s %s\n" "$b" "$t" "$out" ;;
    *"did not finish")printf "CUTOFF %-24s %-6s raise --ticks, this is NOT a failure\n" "$b" "$t" ;;
    "")               printf "CRASH  %-24s %-6s no verdict line at all\n" "$b" "$t" ;;
    *)                printf "RED    %-24s %-6s %s\n" "$b" "$t" "$out" ;;
  esac
}
export -f one

ls scripts/*.txt | xargs -P "$JOBS" -I{} bash -c 'one "$@"' _ {} | sort > /tmp/corpus-run.txt
case "${1:-all}" in
  red) grep '^RED' /tmp/corpus-run.txt | { [ -n "${2:-}" ] && grep "$2" || cat; } ;;
  *)   cat /tmp/corpus-run.txt ;;
esac
echo "---"
for k in GREEN RED CUTOFF CRASH; do printf "%s %s  " "$(grep -c "^$k" /tmp/corpus-run.txt)" "$k"; done; echo
