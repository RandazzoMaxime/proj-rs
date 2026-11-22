#!/usr/bin/env bash
# Run the tests of georust/proj (the Rust binding of libproj) two ways, and compare with PROJ 9.9.0
# (docs/CLIENTS-georust.md):
#   1. the `proj` crate itself, linked to libproj: its unit tests and doc examples, with PROJ 9.9.0
#      and with the libproj of proj-rs swapped in through LD_LIBRARY_PATH (same soname, RUNPATH);
#   2. the same tests and examples on the facade of proj-rs, as a user who changes one line of
#      Cargo.toml: `proj = { package = "proj-rs", ... }` (tools/clients/georust_extract.py).
#
# Usage: tools/clients/georust.sh [WORKDIR]      (default: target/clients/georust)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake,
# pkg-config, network access to github.com and crates.io. The source is pinned by its commit.
set -euo pipefail

TAG=v0.31.0
COMMIT=b035b3c588c3b07ac462262126a37b216d3a37d8
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/georust}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/georust/proj.git "$work/src"
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

rm -rf "$work/ref-prefix"
cmake --install "$ref/build" --prefix "$work/ref-prefix" > /dev/null
"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
export PROJ_DATA=$work/ref-prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

# proj-sys 0.27 prints the pkg-config link path with {:?}, quotes included, which rustc cannot use:
# the path and a RUNPATH go in RUSTFLAGS, so that LD_LIBRARY_PATH can swap the library.
link="-L native=$work/ref-prefix/lib -C link-arg=-Wl,--enable-new-dtags,-rpath,$work/ref-prefix/lib"
linked() {
  (cd "$work/src" && PKG_CONFIG_PATH=$work/ref-prefix/lib/pkgconfig RUSTFLAGS="$link" RUSTDOCFLAGS="$link" \
    CARGO_TARGET_DIR=$work/target cargo test "$@")
}
results() { grep -E '^test ' "$1" | grep -v '^test result' | sed 's/ - should panic//' | sort; }
summary() { grep -E '^test result' "$1" | sed 's/; finished in.*//' | tr '\n' ' '; }

linked -- --test-threads=1 > "$work/linked-ref.txt" 2>&1
linked --doc > "$work/linked-ref-doc.txt" 2>&1
export LD_LIBRARY_PATH=$work/ours/lib
linked -- --test-threads=1 > "$work/linked-ours.txt" 2>&1
linked --doc > "$work/linked-ours-doc.txt" 2>&1
unset LD_LIBRARY_PATH
echo "linked, PROJ 9.9.0: $(summary "$work/linked-ref.txt")$(summary "$work/linked-ref-doc.txt")"
echo "linked, proj-rs:    $(summary "$work/linked-ours.txt")$(summary "$work/linked-ours-doc.txt")"
for kind in "" -doc; do
  diff <(results "$work/linked-ref$kind.txt") <(results "$work/linked-ours$kind.txt") \
    || { echo "the linked runs differ" >&2; exit 1; }
done

rm -rf "$work/facade" && mkdir -p "$work/facade/src" "$work/facade/tests"
cat > "$work/facade/Cargo.toml" <<TOML
[package]
name = "georust-on-proj-rs"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
proj = { package = "proj-rs", path = "$root", features = ["geo-types"] }
geo-types = { version = "0.7.16", features = ["approx"] }

[dev-dependencies]
approx = "0.5"

[workspace]
TOML
python3 -I "$root/tools/clients/georust_extract.py" "$work/src" "$work/facade"
(cd "$work/facade" && CARGO_TARGET_DIR=$work/target-facade cargo test --tests -- --test-threads=1) \
  > "$work/facade.txt" 2>&1 || true
(cd "$work/facade" && CARGO_TARGET_DIR=$work/target-facade cargo test --doc) > "$work/facade-doc.txt" 2>&1 || true
echo "facade proj-rs:     $(summary "$work/facade.txt")$(summary "$work/facade-doc.txt")"
grep -hE '^test .* \.\.\. (FAILED|ignored)' "$work/facade.txt" "$work/facade-doc.txt" || true
