#!/bin/sh
# Package one compiled showcase for the gallery.
#
# Usage: scripts/package_showcase.sh WASM_FILE
#   WASM_FILE  a release build, for example
#              demos/showcase_runner/target/wasm32-unknown-unknown/release/cozy_castle.wasm
#
# Output: demos/gallery/NAME/NAME.js and demos/gallery/NAME/NAME_bg.wasm, where NAME is the
# file name of WASM_FILE without its extension.

set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 WASM_FILE" >&2
  exit 2
fi

wasm_file=$1
if [ ! -f "$wasm_file" ]; then
  echo "No compiled showcase at $wasm_file" >&2
  exit 1
fi

name=$(basename "$wasm_file" .wasm)
out_dir="demos/gallery/$name"

rm -rf "$out_dir"
mkdir -p "$out_dir"
wasm-bindgen "$wasm_file" --out-dir "$out_dir" --target web
wasm-opt -Oz --converge --enable-bulk-memory --enable-nontrapping-float-to-int \
  -o "$out_dir/${name}_bg.wasm" "$out_dir/${name}_bg.wasm"

size=$(wc -c < "$out_dir/${name}_bg.wasm")
echo "Showcase $name: $size bytes ($((size / 1024 / 1024)) MB)"
