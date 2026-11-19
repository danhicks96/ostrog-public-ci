#!/bin/bash
# Make a fresh worktree's Godot project checkable by tools/ci.sh: a fresh worktree has no
# godot/bin/linux/libhf_godot.so (gitignored) and no .godot import cache, and the GDScript check
# then fails with 'Could not find type "SimHost"'. Copies the worktree's (or the main checkout's)
# built library and runs one headless import.
# usage: tools/worktree_godot.sh <worktree>
set -euo pipefail
wt=$1
mkdir -p "$wt/godot/bin/linux"
src="$wt/target/debug/libhf_godot.so"
main=$(git -C "$wt" worktree list --porcelain | sed -n '1s/^worktree //p')
[ -f "$src" ] || src=$main/godot/bin/linux/libhf_godot.so
cp "$src" "$wt/godot/bin/linux/libhf_godot.so"
[ -d "$wt/godot/.godot" ] || (cd "$wt/godot" && timeout 900 ../tools/godot --path . --headless --import > /dev/null 2>&1 || true)
cat "$wt/godot/.godot/extension_list.cfg"
