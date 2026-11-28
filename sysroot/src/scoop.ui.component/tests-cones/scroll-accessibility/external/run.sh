#!/bin/sh
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/scoop-ax-pages.XXXXXX")
pid=''
trap 'if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || :; wait "$pid" 2>/dev/null || :; fi; rm -rf "$work"' EXIT
trap 'exit 1' HUP INT TERM
xcrun clang -fobjc-arc -Wall -Wextra -Werror "$here/client.m" -framework AppKit -framework ApplicationServices -o "$work/client"
scoop build "$here/huge_list.scoop" --release --offline --target-dir "$work/target" -o "$work/list"
"$work/list" > "$work/server.log" &
pid=$!
"$work/client" "$pid"
# Both checkpoints are taken on the UI thread, before the real viewport scroll.
awk '$1 == "counts" {
  if (++n == 1) { keys = $2; builds = $3 }
  else { reads = $2 - keys; built = $3 - builds }
}
END {
  printf "AX queries visited %d keys and built %d views\n", reads, built
  exit !(n == 2 && reads >= 100 && reads < 2000 && built == 0)
}' "$work/server.log"
