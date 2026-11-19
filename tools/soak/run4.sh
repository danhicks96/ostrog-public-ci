#!/bin/bash
# Four opening soaks at once, seeds 1 to 4, from a worktree's release build: how every review's
# soak table was made. Outputs go to OUT/s<N>.txt (the summary lines, including "SOAK events")
# and OUT/s<N>.csv (per game-day); save rings beside them. Then each seed's oldest ring save
# is resumed to the newest's tick and the digests compared ("RESUME seed N: same").
# usage: tools/soak/run4.sh <worktree> <days> <out dir> [extra soak args, e.g. --peoples 16]
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
wt=$1; days=$2; out=$3; shift 3
(cd "$wt" && cargo build --release -q -p hf-cli)
mkdir -p "$out" && cp "$wt/target/release/ostrog" "$out/ostrog"
cd "$out"
for s in 1 2 3 4; do
  ./ostrog soak --opening --seed "$s" --days "$days" --csv "s$s.csv" --ring "ring_s$s" "$@" > "s$s.txt" 2>&1 &
done
wait
grep -h "SOAK" s?.txt
# A resumed game must go on as the continuous one did (#557): each seed's oldest ring save,
# resumed to the newest save's tick, has the newest save's digest.
for s in 1 2 3 4; do
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
