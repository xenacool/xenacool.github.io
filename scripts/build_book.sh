#!/bin/sh
# Build one mdBook variant from a library submodule.
#
# Usage: scripts/build_book.sh SUBMODULE BOOK_NAME VARIANT
#   SUBMODULE  path to the library, for example libs/bevy_gradmesh
#   BOOK_NAME  output directory and URL segment, for example bevy_gradmesh
#   VARIANT    plain (output: BOOK_NAME/) or interactive (output: BOOK_NAME/wasm/)
#
# mdBook copies additional scripts and stylesheets only from the book root, so
# the build runs from a temporary root that holds the book config, a link to the
# book sources, and the web assets for the chosen variant.

set -eu

if [ "$#" -ne 3 ]; then
  echo "Usage: $0 SUBMODULE BOOK_NAME VARIANT" >&2
  exit 2
fi

submodule=$1
book_name=$2
variant=$3
repo_root=$(pwd)
book_dir="$repo_root/$submodule/book"

case "$variant" in
  plain)
    output="$repo_root/$book_name"
    site_url="/$book_name/"
    scripts='"variant_switch.js"'
    styles='"variant_switch.css"'
    ;;
  interactive)
    output="$repo_root/$book_name/wasm"
    site_url="/$book_name/wasm/"
    scripts='"variant_switch.js","book_demos.js"'
    styles='"variant_switch.css","book_demos.css"'
    ;;
  *)
    echo "Unknown variant \"$variant\", expected plain or interactive" >&2
    exit 2
    ;;
esac

if [ ! -f "$book_dir/book.toml" ]; then
  echo "No book.toml at $book_dir. Run: git submodule update --init" >&2
  exit 1
fi

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT

cp "$book_dir/book.toml" "$root/book.toml"
# Patch book.toml for linkcheck2 compatibility (submodule stays clean)
sed -i.bak 's/\[output\.linkcheck\]/[output.linkcheck2]/' "$root/book.toml"
rm -f "$root/book.toml.bak"
# Copy src instead of symlinking; remove broken symlinks (e.g. screenshots/)
cp -R "$book_dir/src" "$root/src"
find "$root/src" -type l ! -exec test -e {} \; -print -delete
cp "$repo_root/js/book_variant_switch_bootstrap.js" "$root/variant_switch.js"
cp "$repo_root/js/book_demos/bootstrap.js" "$root/book_demos.js"
cp "$repo_root/css/book_variant_switch.css" "$root/variant_switch.css"
cp "$repo_root/css/book_demos.css" "$root/book_demos.css"

export MDBOOK_OUTPUT__HTML__SITE_URL="$site_url"
export MDBOOK_OUTPUT__HTML__ADDITIONAL_JS="[$scripts]"
export MDBOOK_OUTPUT__HTML__ADDITIONAL_CSS="[$styles]"
mdbook build "$root" --dest-dir "$root/out"

rm -rf "$output"
mkdir -p "$output"
mv "$root"/out/html/* "$output/"
find "$output" -type f \( -name "*.png" -o -name "*.webp" -o -name "*.jpg" \) -delete
find "$output" -type d -empty -delete
echo "Done: $output/"
