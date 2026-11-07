#!/bin/sh
# Generate the offline API documentation for the installed targets.
set -eu

scoop_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
scoop_plan="$scoop_root/share/scoop/install"
scoop_targets=''
scoop_all_targets=false
scoop_jobs=6
scoop_work="$scoop_root/build/post-install"

scoop_error() {
    printf 'post-install: %s\n' "$*" >&2
    exit 2
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --target|--jobs|--work-dir)
            [ "$#" -ge 2 ] && [ -n "$2" ] || scoop_error "$1 requires a value"
            case "$1" in
                --target)
                    grep -Fxq -- "$2" "$scoop_plan/targets.list" ||
                        scoop_error "target $2 is not provided by this release"
                    case " $scoop_targets " in
                        *" $2 "*) ;;
                        *) scoop_targets="${scoop_targets:+$scoop_targets }$2" ;;
                    esac
                    ;;
                --jobs) scoop_jobs=$2 ;;
                --work-dir) scoop_work=$2 ;;
            esac
            shift 2
            ;;
        --all-targets) scoop_all_targets=true; shift ;;
        --help|-h)
            cat <<'HELP'
Usage: sh post-install.sh [options]
Generate offline API documentation for the installed targets.

  --target TARGET        Document an installed target (repeatable)
  --all-targets          Document every installed target of this release
  --jobs N               Maximum parallel build jobs (default: 6)
  --work-dir PATH        Build cache and intermediates (default: build/post-install)

With no target option, document the release's default target.
HELP
            exit 0
            ;;
        *) scoop_error "unknown option: $1 (see --help)" ;;
    esac
done

case "$scoop_jobs" in ''|*[!0-9]*) scoop_error '--jobs must be positive' ;; esac
[ "$scoop_jobs" -gt 0 ] || scoop_error '--jobs must be positive'
scoop_version=$(head -n 1 "$scoop_root/VERSION")
scoop_installed() {
    [ -f "$scoop_root/lib/$1/scoop.core.slib" ]
}
if [ "$scoop_all_targets" = true ]; then
    [ -z "$scoop_targets" ] || scoop_error '--target and --all-targets are mutually exclusive'
    scoop_targets=''
    while IFS= read -r scoop_target; do
        if scoop_installed "$scoop_target"; then
            scoop_targets="${scoop_targets:+$scoop_targets }$scoop_target"
        else
            printf 'post-install: target %s is not installed; skipping\n' "$scoop_target" >&2
        fi
    done < "$scoop_plan/targets.list"
    [ -n "$scoop_targets" ] ||
        scoop_error "no target package is installed; extract a scoop-$scoop_version-lib-base-<target>.tar.xz archive first"
elif [ -z "$scoop_targets" ]; then
    IFS= read -r scoop_targets < "$scoop_plan/targets.list"
fi

mkdir -p -- "$scoop_work"
scoop_work=$(CDPATH='' cd -- "$scoop_work" && pwd)
mkdir -p "$scoop_work/scratch"
export TMPDIR="$scoop_work/scratch" TMP="$scoop_work/scratch" TEMP="$scoop_work/scratch"

# Target triples contain no whitespace; Cone paths are read one per line.
for scoop_target in $scoop_targets; do
    scoop_installed "$scoop_target" ||
        scoop_error "target $scoop_target is not installed: extract scoop-$scoop_version-lib-base-$scoop_target.tar.xz into $scoop_root first"

    printf 'Generating API documentation for %s\n' "$scoop_target"
    set -- doc
    while IFS= read -r scoop_cone; do
        set -- "$@" "$scoop_root/src/$scoop_cone/Cone.toml"
    done < "$scoop_plan/$scoop_target.list"
    # GUI Cones are documented only when the target's lib-gui package is installed.
    scoop_gui_plan="$scoop_plan/$scoop_target.gui.list"
    if [ -f "$scoop_gui_plan" ]; then
        IFS= read -r scoop_gui_cone < "$scoop_gui_plan"
        if [ -f "$scoop_root/lib/$scoop_target/$scoop_gui_cone.slib" ]; then
            while IFS= read -r scoop_cone; do
                set -- "$@" "$scoop_root/src/$scoop_cone/Cone.toml"
            done < "$scoop_gui_plan"
        fi
    fi
    "$scoop_root/bin/scoop" "$@" --jobs "$scoop_jobs" --target "$scoop_target" \
        --sysroot "$scoop_root" --scoopc "$scoop_root/bin/scoopc" \
        --no-deps --deny-doc-warnings --cache-dir "$scoop_work/cache" \
        --target-dir "$scoop_work/doc-target" --output-dir "$scoop_root/docs/$scoop_target"
done

{
    cat <<'HTML'
<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Scoop built-in API documentation</title></head>
<body><main><h1>Scoop built-in API documentation</h1>
<p>Select the target used to compile your application.
All pages, source text, and search resources are included locally.</p><ul>
HTML
    while IFS= read -r scoop_target; do
        if [ -f "$scoop_root/docs/$scoop_target/index.html" ]; then
            printf '<li><a href="%s/index.html">%s</a></li>\n' "$scoop_target" "$scoop_target"
        fi
    done < "$scoop_plan/targets.list"
    printf '</ul></main></body></html>\n'
} > "$scoop_root/docs/index.html"
printf 'API documentation is ready in %s\n' "$scoop_root"
