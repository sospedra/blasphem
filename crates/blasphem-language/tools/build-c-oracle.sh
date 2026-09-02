#!/bin/sh
set -eu

# Build the temporary fixture oracle against the audited ELDC snapshot.
# Usage: crates/eldc/tools/build-c-oracle.sh /tmp/eldc-c-oracle

output=${1:?"pass a temporary output path"}
upstream_dir=${ELDC_UPSTREAM_DIR:-/private/tmp/eldc-audit-20260902/src/eldc}
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

check_digest() {
    expected=$1
    file=$2
    actual=$(shasum -a 256 "$file" | awk '{print $1}')
    if [ "$actual" != "$expected" ]; then
        printf '%s\n' "digest mismatch: $file" >&2
        exit 1
    fi
}

check_digest 59c36f7ebe1fe972ac4d3553b293a67b0aa3506589f853392fdf58675edb0a2d "$upstream_dir/eld_core.c"
check_digest 4f9f3d9741e5f594b0a50da9bf1d26cfba2b8f049a1b75627114a6cc9c0dfe64 "$upstream_dir/large_db.h"
check_digest e620b9feb08eb32ce751a7148a51b19c5eb2774d2dff74f5dd2d1363184df23b "$upstream_dir/eld_unicode_bits.h"
check_digest 97722a4d9765e609631ce527ff42b27a4e589d7e673d17e8bf1da68068da1d2b "$upstream_dir/eld_tolower.h"
check_digest fcc72989c8655856501c6382019d2b9335c71cb89f38f59ffddd28b0019a34f0 "$upstream_dir/eld_iso639_2t.h"

"${CC:-cc}" \
    -std=c11 \
    -O2 \
    -Wall \
    -Wextra \
    -Werror \
    -DELD_CORE_PATH=\"$upstream_dir/eld_core.c\" \
    "$script_dir/c-oracle.c" \
    -lm \
    -o "$output"
