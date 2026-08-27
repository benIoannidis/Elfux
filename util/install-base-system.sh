#!/bin/sh
set -eu

sysroot=${1:-sysroot}
repo_json=${2:-/home/ben/elfux_package_repo.json}
package=${3:-base-system}
cache_dir=${ELFUX_PACKAGE_CACHE:-.pkg-cache}

mkdir -p "$sysroot" "$cache_dir"

package_order=$(mktemp)
trap 'rm -f "$package_order"' EXIT

python - "$repo_json" "$package" <<'PY' > "$package_order"
import json
import sys
from pathlib import Path

repo = json.loads(Path(sys.argv[1]).read_text())["packages"]
root = sys.argv[2]
seen = set()
order = []

def visit(name):
    if name in seen:
        return
    seen.add(name)
    for dep in repo[name].get("dependencies", []):
        visit(dep)
    order.append(name)

visit(root)
for name in order:
    meta = repo[name]
    format_name = meta.get("format", "tar.gz")
    print("|".join([
        name,
        meta.get("url", ""),
        format_name,
        str(meta.get("strip_components", 1)),
        json.dumps(meta.get("binary_links", {}), separators=(",", ":")),
    ]))
PY

while IFS='|' read -r name url format strip links
do
    if [ "$format" = meta ]; then
        continue
    fi

    archive=$cache_dir/${url##*/}
    archive=${archive%%\?*}

    if [ ! -s "$archive" ]; then
        printf '[BASE] Downloading %s\n' "$name"
        curl -L --fail --show-error --output "$archive" "$url"
    else
        printf '[BASE] Using cached %s\n' "$name"
    fi

    case "$format" in
        binary)
            dest=$(python - "$links" <<'PY'
import json
import sys
links = json.loads(sys.argv[1])
print(next(iter(links.values()), "/bin/unknown"))
PY
)
            mkdir -p "$sysroot/$(dirname "${dest#/}")"
            cp "$archive" "$sysroot/${dest#/}"
            chmod +x "$sysroot/${dest#/}"
            ;;
        tar.gz|tgz)
            tar --no-same-owner --skip-old-files --exclude=.BUILDINFO --exclude=.MTREE --exclude=.PKGINFO --exclude=.INSTALL --strip-components="$strip" -xzf "$archive" -C "$sysroot"
            ;;
        tar.xz)
            tar --no-same-owner --skip-old-files --exclude=.BUILDINFO --exclude=.MTREE --exclude=.PKGINFO --exclude=.INSTALL --strip-components="$strip" -xJf "$archive" -C "$sysroot"
            ;;
        tar.zst|pkg.tar.zst)
            tar --zstd --no-same-owner --skip-old-files --exclude=.BUILDINFO --exclude=.MTREE --exclude=.PKGINFO --exclude=.INSTALL --strip-components="$strip" -xf "$archive" -C "$sysroot"
            ;;
        *)
            printf '[BASE] Unsupported package format for %s: %s\n' "$name" "$format" >&2
            exit 1
            ;;
    esac

    python - "$sysroot" "$links" <<'PY'
import json
import os
import sys
from pathlib import Path

sysroot = Path(sys.argv[1])
links = json.loads(sys.argv[2])
for source, target in links.items():
    source_path = sysroot / source.lstrip("/")
    target_path = sysroot / target.lstrip("/")
    if not source_path.exists() or target_path.exists():
        continue
    target_path.parent.mkdir(parents=True, exist_ok=True)
    rel_source = os.path.relpath(source_path, target_path.parent)
    target_path.symlink_to(rel_source)
PY
done < "$package_order"

mkdir -p "$sysroot/var/lib/elfpkg/installed" "$sysroot/lib64"
while IFS='|' read -r name _url _format _strip _links
do
    touch "$sysroot/var/lib/elfpkg/installed/$name.list"
done < "$package_order"
touch "$sysroot/var/lib/elfpkg/installed/$package.list"
if [ -e "$sysroot/usr/lib/ld-linux-x86-64.so.2" ] && [ ! -e "$sysroot/lib64/ld-linux-x86-64.so.2" ]; then
    ln -s ../usr/lib/ld-linux-x86-64.so.2 "$sysroot/lib64/ld-linux-x86-64.so.2"
fi
if [ -e "$sysroot/usr/bin/bash" ] && [ ! -e "$sysroot/bin/bash" ]; then
    ln -s ../usr/bin/bash "$sysroot/bin/bash"
fi
if [ -e "$sysroot/usr/bin/bash" ]; then
    rm -f "$sysroot/bin/sh"
    ln -s ../usr/bin/bash "$sysroot/bin/sh"
fi