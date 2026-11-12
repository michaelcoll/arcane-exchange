#!/usr/bin/env bash
# Checks how the built app links its Swift libraries, two ways a launch breaks that the tests
# (no host app) never see:
#
# 1. A Swift module compiled into more than one binary. Each copy has its own globals: a
#    singleton configured through one copy is unset in the other (`Clerk.shared` trapped at
#    launch, `ImagePipeline.shared` silently ignored). Xcode makes this easy: a package product
#    linked by a single target is linked statically, dependencies included, even when another
#    target already gets that dependency as a dynamic framework. A module is identified by the
#    nominal type descriptors (`$s<len><Module>…Mn`) a binary defines.
#
# 2. An `@rpath` library missing from the bundle. A Debug build adds DerivedData to the rpaths,
#    so the simulator still finds a framework the app forgot to embed; a device does not, and
#    dyld kills the app before `main`, with no log.
#
# Usage: check-app-linking.sh <path/to/ArcaneExchange.app>
set -euo pipefail

app=${1:?usage: $0 <path/to/App.app>}
[ -d "$app" ] || { echo "no app bundle at $app" >&2; exit 2; }
status=0

# Every Mach-O of the bundle, test bundles aside: a stale one can linger in DerivedData.
binaries=()
while IFS= read -r -d '' file; do
    case "$file" in *.xctest/*) continue ;; esac
    if file -b "$file" | grep -q 'Mach-O'; then
        binaries+=("$file")
    fi
done < <(find "$app" -type f -perm -u+x -print0)

# 1. Modules compiled twice.
listing=$(
    for binary in "${binaries[@]}"; do
        nm -U -j "$binary" 2>/dev/null \
            | sed -nE 's/^_\$s([0-9]+)([A-Za-z_][A-Za-z0-9_]*).*Mn$/\1 \2/p' \
            | while read -r length name; do
                # The mangled length bounds the module name: `8ClerkKit5ClerkC` is `ClerkKit`.
                echo "${name:0:length}"
            done \
            | sort -u \
            | sed "s|\$|	${binary#"$app"/}|"
    done
)
# A type declared in an extension of an SDK type mangles under the SDK module (`SwiftUI`): those
# are not copies of the module.
sdk=$(xcrun --sdk iphonesimulator --show-sdk-path)
duplicates=$(
    printf '%s\n' "$listing" | cut -f1 | sort | uniq -d | while read -r module; do
        [ -n "$module" ] || continue
        [ -e "$sdk/System/Library/Frameworks/$module.framework" ] && continue
        [ -e "$sdk/usr/lib/swift/$module.swiftmodule" ] && continue
        echo "$module"
    done
)
if [ -n "$duplicates" ]; then
    echo "Swift modules compiled into more than one binary of $(basename "$app"):" >&2
    for module in $duplicates; do
        echo "  $module:" >&2
        printf '%s\n' "$listing" | awk -F'\t' -v m="$module" '$1 == m { print "    " $2 }' >&2
    done
    status=1
fi

# 2. `@rpath` libraries missing from the bundle. dyld searches the rpaths of the whole load
# chain, so the search directories are every relative rpath (`@executable_path`,
# `@loader_path`) of the bundle, resolved; absolute ones point into the build machine.
# The Swift runtime (`libswift*`) ships with the OS.
search_dirs=$(
    for binary in "${binaries[@]}"; do
        dir=$(dirname "$binary")
        executable_dir=$app
        case "$binary" in "$app"/PlugIns/*.appex/*) executable_dir=${binary%%.appex/*}.appex ;; esac
        otool -l "$binary" | awk '/cmd LC_RPATH/ { getline; getline; print $2 }' | grep '^@' \
            | sed -e "s|^@executable_path|$executable_dir|" -e "s|^@loader_path|$dir|" || true
    done | sort -u
)
for binary in "${binaries[@]}"; do
    install_name=$(otool -D "$binary" | sed -n 2p)
    while read -r library; do
        [ "$library" = "$install_name" ] && continue
        relative=${library#@rpath/}
        case "$relative" in libswift*) continue ;; esac
        found=false
        for search_dir in $search_dirs; do
            [ -e "$search_dir/$relative" ] && found=true && break
        done
        if ! $found; then
            echo "${binary#"$app"/} loads $library, which is not in the bundle (not embedded?)" >&2
            status=1
        fi
    done < <(otool -L "$binary" | awk 'NR > 1 && $1 ~ /^@rpath\// { print $1 }')
done

if [ "$status" -eq 0 ]; then
    echo "$(basename "$app"): no Swift module compiled twice, every @rpath library embedded (${#binaries[@]} binaries)."
fi
exit "$status"
