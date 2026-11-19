#!/usr/bin/env bash
# Where ostrog.gdextension loads the extension library on this platform, and cargo's name
# for it: prints "<bin dir> <file>", e.g. "linux libhf_godot.so". Optional argument: a
# `uname -s` value to answer for (CI checks the Windows answer on Linux, #198).
case "${1:-$(uname -s)}" in
  MINGW*|MSYS*|CYGWIN*) echo "windows hf_godot.dll" ;;
  *) echo "linux libhf_godot.so" ;;
esac
