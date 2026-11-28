#!/bin/bash
# Opening soaks at once, seeds 1 to 4 (or SEEDS), from a worktree's release build: how every review's
# soak table was made. Outputs go to OUT/s<N>.txt (the summary lines, including "SOAK events")
# and OUT/s<N>.csv (per game-day); save rings beside them. Then each seed's oldest ring save
# is resumed to the newest's tick and the digests compared ("RESUME seed N: same").
# With `--alive` among the extra args, every seed'"'"'s "ALIVE key value" lines are kept in OUT/alive.txt
# ("<key> s<seed> <value>", sorted by key) and printed one key to a line, the seeds side by side.
# usage: [SEEDS="1 2 3 4"] tools/soak/run4.sh <worktree> <days> <out dir> [extra soak args, e.g. --peoples 16]
# (the script keeps its name; SEEDS is the list of seeds, default "1 2 3 4")
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
wt=$1; days=$2; out=$3; shift 3
seeds=${SEEDS:-"1 2 3 4"}
(cd "$wt" && cargo build --release -q -p hf-cli)
mkdir -p "$out" && cp "$wt/target/release/ostrog" "$out/ostrog"
cd "$out"
for s in $seeds; do
  ./ostrog soak --opening --seed "$s" --days "$days" --csv "s$s.csv" --ring "ring_s$s" "$@" > "s$s.txt" 2>&1 &
done
wait
grep -h "SOAK" s[0-9]*.txt
# The alive report of every seed (only when the soak ran with --alive), key by key.
if grep -q "^ALIVE " s[0-9]*.txt 2>/dev/null; then
  for f in s[0-9]*.txt; do
    awk -v seed="${f%.txt}" '$1 == "ALIVE" { print $2, seed, $3 }' "$f"
  done | sort -k1,1 -s > alive.txt
  awk '{ v[$1] = v[$1] " " $2 "=" $3; if (!($1 in o)) { o[$1] = ++n; k[n] = $1 } }
       END { for (i = 1; i <= n; i++) print k[i] ":" v[k[i]] }' alive.txt
fi
# A resumed game must go on as the continuous one did (#557): each seed's oldest ring save,
# resumed to the newest save's tick, has the newest save's digest.
for s in $seeds; do
  (
    rows=$(for d in ring_s$s.*.hfsave; do ./ostrog resume "$d" --ticks 0 | sed "s|^|$d |"; done | sort -k3 -n)
    [ "$(echo "$rows" | wc -l)" -ge 2 ] || { echo "RESUME seed $s: fewer than two saves, skipped"; exit 0; }
    old=$(echo "$rows" | head -1); new=$(echo "$rows" | tail -1)
    ticks=$(( $(echo "$new" | cut -d' ' -f3) - $(echo "$old" | cut -d' ' -f3) ))
    got=$(./ostrog resume "$(echo "$old" | cut -d' ' -f1)" --ticks "$ticks" | cut -d' ' -f4)
    if [ "$got" = "$(echo "$new" | cut -d' ' -f5)" ]; then
      echo "RESUME seed $s: same ($ticks ticks from $(echo "$old" | cut -d' ' -f1))"
    else
      echo "RESUME seed $s: DIFFERS ($ticks ticks from $(echo "$old" | cut -d' ' -f1))"
    fi
  ) &
done
wait
