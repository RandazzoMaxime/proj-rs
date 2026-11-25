#!/usr/bin/env bash
# Run the test suites of the R packages sf and terra, real clients of libproj, against PROJ
# 9.9.0 and against the C library of proj-rs (docs/CLIENTS-r.md).
#
# R is the language statistics of geography has chosen: sf (simple features) and terra (raster
# and vector geographic data) are its two geospatial packages, both built on GDAL. GDAL does the
# libproj work (OGRSpatialReference parsing, OGRCreateCoordinateTransformation, proj_pipelines
# strings), and terra calls libproj itself in one own place: proj_pipelines, which lists the
# coordinate operations of a CRS pair through the C API, and projNetwork's warning that datum
# grids could not be downloaded. The packages' compiled code links no libproj: the GDAL of the
# system does, the same soname the two passes swap.
#
# The suites are the R-level tests of each package: the 25 *.R files of the sf/tests directory
# (R CMD BATCH --no-timing, one output each), and terra's tinytest suite (981 results). Each
# output is compared between the two passes after normalization: the timing lines of R are
# dropped, and the "Cannot open https://cdn.proj.org/...: Network functionality not available"
# lines too (both libraries emit them when a datum grid on neither disk nor network is tried,
# and how many depends on the API path, which logs through R only see as a count).
#
# Usage: tools/clients/r.sh [WORKDIR]          (default: target/clients/r)
# R_HOME uses an existing R >= 4.5 instead of building the pinned one.
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C
# compiler, network access to CRAN the first time the workdir runs. System libraries, as root:
# libblas, liblapack, libreadline, libpcre2, libcurl, libicu, gfortran (for R), libgdal-dev,
# libgeos-dev, libsqlite3-dev, libudunits2-dev, libxml2-dev (for sf and its dependencies).
# Both passes read the same data directory: the share/proj of the reference install first, and
# the tests fixtures of proj-rs, with the network of libproj off, so a difference comes from the
# library.
set -euo pipefail

R_VERSION=4.5.0
SF_VERSION=1.1-3
SF_SHA256=58b6ac4fc001ca03d358c1c227f86a52a18822e52abb56a25b548b2f8b74cd59
TERRA_VERSION=1.9-60
TERRA_SHA256=a920d79dbab104816b00efd1738af570c019bf4dfe457375bcf52151091bcb8e
LWGEOM_VERSION=0.2-17
LWGEOM_SHA256=f25a48ce47546bd37ddc92c5455573e27f712cecc6042ec9588ecf1438e29acc
CRAN=https://cran.r-project.org

root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/r}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

download() { # download <url> <sha256|-> <path>
  local url=$1 sha=$2 path=$3
  [ -s "$path" ] && return 0
  curl -fsSL "$url" -o "$path"
  if [ "$sha" != - ]; then
    echo "$sha  $path" | sha256sum --check --quiet
  fi
}

Rlibdir=$work/rlib

# R, built once from the version-pinned CRAN source into the workdir: the same one hosts both
# passes. An R already on the system skips the build.
if [ -z "${R_HOME:-}" ]; then
  Rbin=$work/R-$R_VERSION/usr/lib/R/bin/R
else
  Rbin=$R_HOME/bin/R
fi
if [ ! -x "$Rbin" ]; then
  echo "building R $R_VERSION from $CRAN/src/base/R-4 (about half an hour)"
  download "$CRAN/src/base/R-4/R_$R_VERSION.tar.gz" - "$work/R_$R_VERSION.tar.gz"
  rm -rf "$work/R-$R_VERSION-src"
  tar -xzf "$work/R_$R_VERSION.tar.gz" -C "$work"
  mv "$work/R-$R_VERSION" "$work/R-$R_VERSION-src"
  (cd "$work/R-$R_VERSION-src" && ./configure --prefix="$work/R-$R_VERSION/usr" --with-x=no \
    --with-blas --with-lapack --enable-R-shlib > "$work/r-configure.log" 2>&1 \
    && make -j"$(nproc)" > "$work/r-make.log" 2>&1 \
    && make install > "$work/r-install.log" 2>&1) \
    || { echo "R does not build (see $work/r-*.log)" >&2; exit 1; }
fi
export R_LIBS=$Rlib
rp() { # rp <file of R code>
  "$Rbin" --vanilla --no-echo --file="$1"
}

# sf, terra and lwgeom, from their pinned CRAN sources, with their CRAN dependencies.
if [ ! -d "$Rlibdir/sf" ] || [ ! -d "$Rlibdir/terra" ]; then
  mkdir -p "$Rlibdir"
  download "$CRAN/src/contrib/Archive/sf/sf_$SF_VERSION.tar.gz" "$SF_SHA256" "$work/sf.tar.gz"
  download "$CRAN/src/contrib/Archive/terra/terra_$TERRA_VERSION.tar.gz" "$TERRA_SHA256" "$work/terra.tar.gz"
  download "$CRAN/src/contrib/Archive/lwgeom/lwgeom_$LWGEOM_VERSION.tar.gz" "$LWGEOM_SHA256" "$work/lwgeom.tar.gz"
  rm -rf "$work/src-sf" "$work/src-terra" "$work/src-lwgeom"
  (cd "$work" && tar -xzf sf.tar.gz && mv sf src-sf && tar -xzf terra.tar.gz \
    && mv terra src-terra && tar -xzf lwgeom.tar.gz && mv lwgeom src-lwgeom)
  cat > "$work/install.R" <<EOF
options(repos = c(CRAN = "$CRAN"), Ncpus = parallel::detectCores())
install.packages(c(terra = "$work/terra.tar.gz", lwgeom = "$work/lwgeom.tar.gz",
                   sf = "$work/sf.tar.gz"), repos = NULL, type = "source")
EOF
  rp "$work/install.R" > "$work/packages.log" 2>&1 \
    || { echo "the packages do not install ($work/packages.log)" >&2; exit 1; }
fi

# The two libproj prefixes: the reference install, and ours. The GDAL the packages link resolves
# libproj.so.25 through LD_LIBRARY_PATH, the same soname both prefixes install.
rm -rf "$work/ref-prefix" "$work/ours-prefix"
cmake --install "$ref/build" --prefix "$work/ref-prefix" > /dev/null
"$root/capi/tests/dist.sh" --release "$work/ours-prefix" > /dev/null
projdata=$work/ref-prefix/share/proj:$root/data/tests

run() { # run <name> <libdir>
  local name=$1 libdir=$2 out=$work/out-$name
  rm -rf "$out" && mkdir -p "$out"
  (
    set -u
    export LD_LIBRARY_PATH="$libdir"
    export PROJ_DATA="$projdata"
    export PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
    export PROJ_NETWORK=OFF
    export R_LIBS="$Rlibdir"
    for f in "$work"/src-sf/tests/*.R; do
      (cd "$work/src-sf/tests" && "$Rbin" CMD BATCH --no-timing "$f" "$out/sf-$(basename "$f").Rout" \
        2>/dev/null || echo "SF-FAIL $(basename "$f")" >> "$out/failures.txt")
    done
    "$Rbin" --vanilla -f "$work/src-terra/tests/tinytest.R" > "$out/terra-tinytest.log" 2>&1 \
      || echo "TERRA-FAIL" >> "$out/failures.txt"
  )
}

run ref "$work/ref-prefix/lib"
run ours "$work/ours-prefix/lib"

norm() { # norm <file>
  python3 - "$1" <<'PYEOF'
import re, sys
noise = re.compile(r"Cannot open https://cdn\.proj\.org/\S+: "
                  r"Network functionality not available")
duration = re.compile(r"( +[0-9]+(\.[0-9]+)?(ms|s)\b|^[0-9]+(\.[0-9]+)?(ms|s)$)")
for raw in open(sys.argv[1], errors="replace"):
    line = noise.sub("", raw.rstrip("\n"))
    line = duration.sub("", line)
    line = line.strip()
    if line:
        print(line)
PYEOF
}

same=0
for file in "$work"/out-ref/*; do
  base=$(basename "$file")
  [ -f "$work/out-ours/$base" ] || { echo "MISSING $base"; continue; }
  norm "$file" > "$work/.ref.norm"
  norm "$work/out-ours/$base" > "$work/.ours.norm"
  if cmp -s "$work/.ref.norm" "$work/.ours.norm"; then
    same=$((same + 1))
  else
    echo "DIFFERS $base"
  fi
done
rm -f "$work/.ref.norm" "$work/.ours.norm"
echo "identical after normalization: $same of $(ls "$work"/out-ref | wc -l) files"
grep -o "All ok, [0-9]* results" "$work/out-ours/terra-tinytest.log" \
  || echo "terra: see $work/out-ours/terra-tinytest.log"
