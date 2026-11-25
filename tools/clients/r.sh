#!/usr/bin/env bash
# Run the test suites of the R packages sf and terra, real clients of libproj, against PROJ
# 9.9.0 and against the C library of proj-rs (docs/CLIENTS-r.md).
#
# R is the language statistics of geography has chosen: sf (simple features) and terra (raster
# and vector geographic data) are its two geospatial packages, both built on GDAL. GDAL does the
# libproj work (OGRSpatialReference parsing, OGRCreateCoordinateTransformation, proj_pipelines
# strings), and terra calls libproj itself in one own place: proj_pipelines, which lists the
# coordinate operations of a CRS pair through the C API, and projNetwork's warning that datum
# grids could not be downloaded. The packages' compiled code links the GDAL of the system, which
# links libproj.so.25: the same soname the two passes swap.
#
# The suites are the R-level tests of each package: the *.R files of the sf/tests directory
# (R CMD BATCH --no-timing, one output each), and terra's tinytest suite. Each output is compared
# between the two passes after one normalization: the durations R and tinytest print are dropped.
# Everything else is compared as written, the "Cannot open <url>: <error>" lines libproj logs
# included, and their count is reported per pass.
#
# Usage: tools/clients/r.sh [WORKDIR]          (default: target/clients/r)
# R_HOME uses an existing R >= 4.5 instead of building the pinned one.
# Exit status: 0 when the two passes agree, 1 when they differ or a suite did not run.
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C, C++
# and Fortran compiler, network access to CRAN and to Posit Package Manager the first time the
# workdir runs. System libraries, as root: libblas, liblapack, libreadline, libpcre2, libcurl,
# libicu, gfortran (for R), libgdal-dev, libgeos-dev, libsqlite3-dev, libudunits2-dev,
# libxml2-dev, libssl-dev, libuv1-dev (for sf, terra, lwgeom and their dependencies; fs, which
# testthat needs, builds against libuv).
# Both passes read the same data directory: the share/proj of the reference install first, and
# the tests fixtures of proj-rs, with the network of libproj off, so a difference comes from the
# library.
set -euo pipefail

R_VERSION=4.5.0
R_SHA256=3b33ea113e0d1ddc9793874d5949cec2c7386f66e4abfb1cef9aec22846c3ce1
SF_VERSION=1.1-3
SF_SHA256=58b6ac4fc001ca03d358c1c227f86a52a18822e52abb56a25b548b2f8b74cd59
TERRA_VERSION=1.9-60
TERRA_SHA256=a920d79dbab104816b00efd1738af570c019bf4dfe457375bcf52151091bcb8e
LWGEOM_VERSION=0.2-17
LWGEOM_SHA256=f25a48ce47546bd37ddc92c5455573e27f712cecc6042ec9588ecf1438e29acc
CRAN=https://cran.r-project.org
# The dependencies come from CRAN as it was on that day: Posit Package Manager serves each dated
# snapshot unchanged, so the versions of Rcpp, s2, units, wk, classInt, DBI, tinytest... are
# fixed by the date (they are listed in out-*/packages.txt).
SNAPSHOT=https://packagemanager.posit.co/cran/2026-11-24
# Besides what sf, terra and lwgeom need: the harness of terra's suite, the harness of sf's
# testthat.R, ncdf4, which terra's suite loads without a guard, tibble, which read_sf returns,
# sp and dplyr, which 8 and 4 of sf's test scripts load.
EXTRA_PACKAGES="tinytest testthat ncdf4 tibble sp dplyr"
extra_r=$(printf '"%s", ' $EXTRA_PACKAGES)
extra_r=${extra_r%, }

root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/r}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

download() { # download <sha256> <path> <url>...: the first URL that answers, checked
  local sha=$1 path=$2 url
  shift 2
  if [ ! -s "$path" ]; then
    for url in "$@"; do
      curl -fsSL "$url" -o "$path.part" && break
    done
    mv "$path.part" "$path"
  fi
  echo "$sha  $path" | sha256sum --check --quiet
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
  echo "building R $R_VERSION from $CRAN/src/base/R-4 (about twenty minutes)"
  download "$R_SHA256" "$work/R-$R_VERSION.tar.gz" "$CRAN/src/base/R-4/R-$R_VERSION.tar.gz"
  rm -rf "$work/R-$R_VERSION-src"
  tar -xzf "$work/R-$R_VERSION.tar.gz" -C "$work"
  mv "$work/R-$R_VERSION" "$work/R-$R_VERSION-src"
  (cd "$work/R-$R_VERSION-src" && ./configure --prefix="$work/R-$R_VERSION/usr" --with-x=no \
    --with-blas --with-lapack --enable-R-shlib > "$work/r-configure.log" 2>&1 \
    && make -j"$(nproc)" > "$work/r-make.log" 2>&1 \
    && make install > "$work/r-install.log" 2>&1) \
    || { echo "R does not build (see $work/r-*.log)" >&2; exit 1; }
fi
export R_LIBS=$Rlibdir
rp() { # rp <file of R code>
  "$Rbin" --vanilla --no-echo --file="$1"
}

# sf, terra and lwgeom from their CRAN sources, checked by sha256 (the current file, or the
# archived one once a newer version is out); everything they need from the dated snapshot.
installed=1
for package in sf terra lwgeom $EXTRA_PACKAGES; do
  [ -d "$Rlibdir/$package" ] || installed=0
done
if [ "$installed" = 0 ]; then
  mkdir -p "$Rlibdir"
  for spec in "sf $SF_VERSION $SF_SHA256" "terra $TERRA_VERSION $TERRA_SHA256" \
              "lwgeom $LWGEOM_VERSION $LWGEOM_SHA256"; do
    set -- $spec
    download "$3" "$work/$1.tar.gz" "$CRAN/src/contrib/$1_$2.tar.gz" \
      "$CRAN/src/contrib/Archive/$1/$1_$2.tar.gz"
  done
  rm -rf "$work/src-sf" "$work/src-terra" "$work/src-lwgeom"
  (cd "$work" && tar -xzf sf.tar.gz && mv sf src-sf && tar -xzf terra.tar.gz \
    && mv terra src-terra && tar -xzf lwgeom.tar.gz && mv lwgeom src-lwgeom)
  cat > "$work/install.R" <<EOF
options(repos = c(CRAN = "$SNAPSHOT"), Ncpus = parallel::detectCores())
which <- c("Depends", "Imports", "LinkingTo")
db <- available.packages()
pinned <- c("terra", "sf", "lwgeom")
need <- unique(c(unlist(tools::package_dependencies(c(pinned, $extra_r), db = db,
                                                     which = which, recursive = TRUE)),
                 $extra_r))
need <- setdiff(need, c(pinned, rownames(installed.packages())))
install.packages(need, type = "source", dependencies = which)
missing <- setdiff(need, rownames(installed.packages()))
if (length(missing)) stop("not installed: ", paste(missing, collapse = " "))
install.packages(c("$work/terra.tar.gz", "$work/sf.tar.gz", "$work/lwgeom.tar.gz"),
                 repos = NULL, type = "source")
stopifnot(packageVersion("sf") == "$SF_VERSION", packageVersion("terra") == "$TERRA_VERSION",
          packageVersion("lwgeom") == "$LWGEOM_VERSION")
EOF
  rp "$work/install.R" > "$work/packages.log" 2>&1 \
    || { echo "the packages do not install ($work/packages.log)" >&2; exit 1; }
fi
cat > "$work/versions.R" <<'EOF'
ip <- installed.packages(lib.loc = .libPaths()[1])
writeLines(paste(ip[, "Package"], ip[, "Version"]))
EOF

# The two libproj prefixes: the reference install, and ours. The GDAL the packages link resolves
# libproj.so.25 through LD_LIBRARY_PATH, the same soname both prefixes install.
rm -rf "$work/ref-prefix" "$work/ours-prefix"
cmake --install "$ref/build" --prefix "$work/ref-prefix" > /dev/null
"$root/capi/tests/dist.sh" --release "$work/ours-prefix" > /dev/null
projdata=$work/ref-prefix/share/proj:$root/data/tests

# The libproj the R process maps once sf and terra are loaded (GDAL, which they link, loads it):
# every mapped libproj must be the one of the pass, else the pass would compare a library with
# itself. bin/R sources etc/ldpaths, which puts R_LD_LIBRARY_PATH in front of LD_LIBRARY_PATH:
# the pass sets both.
cat > "$work/libcheck.R" <<'EOF'
suppressPackageStartupMessages({ library(sf); library(terra) })
invisible(sf::st_crs(4326)$wkt)
expected <- normalizePath(Sys.getenv("PASS_LIBDIR"))
maps <- unique(sub("^.* ", "", grep("libproj", readLines("/proc/self/maps"), value = TRUE)))
real <- unique(normalizePath(maps))
cat("expected libproj directory:", expected, "\n")
cat(paste("mapped:", maps, "->", real), sep = "\n")
cat("sf:", as.character(packageVersion("sf")), " terra:", as.character(packageVersion("terra")),
    " GDAL:", sf::sf_extSoftVersion()[["GDAL"]], " PROJ (sf):", sf::sf_extSoftVersion()[["PROJ"]],
    " PROJ (terra):", terra::gdal(lib = "proj"), "\n")
ok <- length(real) > 0 && all(dirname(real) == expected)
cat(if (ok) "libproj check: ok\n" else "libproj check: FAILED\n")
if (!ok) quit(status = 1)
EOF

echo 'options(bitmapType = "cairo")' > "$work/Rprofile"

run() { # run <name> <libdir>
  local name=$1 libdir=$2
  local out=$work/out-$name
  rm -rf "$out" && mkdir -p "$out"
  (
    export LD_LIBRARY_PATH="$libdir"
    export R_LD_LIBRARY_PATH="$libdir:$(dirname "$(dirname "$Rbin")")/lib"
    export PASS_LIBDIR="$libdir"
    export PROJ_DATA="$projdata"
    export PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
    export PROJ_NETWORK=OFF
    export R_LIBS="$Rlibdir"
    rp "$work/versions.R" > "$out/packages.txt"
    rp "$work/libcheck.R" > "$out/libcheck.txt" 2>&1 \
      || { cat "$out/libcheck.txt" >&2; echo "pass $name: wrong libproj" >&2; exit 1; }
    # The scripts write files next to themselves (geom.gpkg...): each pass runs a fresh copy
    # of the tests directory, and --vanilla keeps R CMD BATCH from saving or restoring .RData.
    rm -rf "$work/run-$name" && mkdir -p "$work/run-$name"
    tar -xzf "$work/sf.tar.gz" -C "$work/run-$name" sf/tests
    local tests=$work/run-$name/sf/tests
    for f in "$tests"/*.R; do
      (cd "$tests" && "$Rbin" CMD BATCH --vanilla --no-timing "$f" "$out/sf-$(basename "$f").Rout" \
        2>/dev/null || echo "SF-FAIL $(basename "$f")" >> "$out/failures.txt")
    done
    # An R built without X11 defaults png() to Xlib, which test_plot.R cannot open: the suite
    # runs with the user profile below alone, which picks cairo.
    R_PROFILE_USER="$work/Rprofile" "$Rbin" --no-save --no-restore --no-site-file --no-environ \
      -f "$work/src-terra/tests/tinytest.R" > "$out/terra-tinytest.log" 2>&1 \
      || echo "TERRA-FAIL" >> "$out/failures.txt"
  )
}

run ref "$work/ref-prefix/lib"
run ours "$work/ours-prefix/lib"
echo "== libproj loaded by each pass"
for name in ref ours; do
  echo "-- $name"
  cat "$work/out-$name/libcheck.txt"
done

norm() { # norm <file>: the durations of R and tinytest dropped, nothing else
  python3 - "$1" <<'PYEOF'
import re, sys
duration = re.compile(r"( +[0-9]+(\.[0-9]+)?(ms|s)\b|^[0-9]+(\.[0-9]+)?(ms|s)$)")
for raw in open(sys.argv[1], errors="replace"):
    line = duration.sub("", raw.rstrip("\n")).strip()
    if line:
        print(line)
PYEOF
}

status=0
same=0
total=0
for base in $( (ls "$work/out-ref"; ls "$work/out-ours") | grep -v -x -e libcheck.txt | sort -u); do
  total=$((total + 1))
  if [ ! -f "$work/out-ref/$base" ] || [ ! -f "$work/out-ours/$base" ]; then
    echo "MISSING $base (in $( [ -f "$work/out-ref/$base" ] && echo ref || echo ours) only)"
    status=1
    continue
  fi
  norm "$work/out-ref/$base" > "$work/.ref.norm"
  norm "$work/out-ours/$base" > "$work/.ours.norm"
  if cmp -s "$work/.ref.norm" "$work/.ours.norm"; then
    same=$((same + 1))
  else
    echo "DIFFERS $base"
    { diff "$work/.ref.norm" "$work/.ours.norm" || true; } | head -20
    status=1
  fi
done
rm -f "$work/.ref.norm" "$work/.ours.norm"
echo "identical after normalization: $same of $total files"

expected_sf=$(tar -tzf "$work/sf.tar.gz" | grep -c '^sf/tests/[^/]*\.R$')
for name in ref ours; do
  out=$work/out-$name
  sf=$(ls "$out"/sf-*.Rout 2>/dev/null | wc -l)
  terra=$(grep -o "All ok, [0-9]* results" "$out/terra-tinytest.log" || true)
  cannot=$(cat "$out"/* | grep -c "Cannot open " || true)
  failed=$( [ -f "$out/failures.txt" ] && tr '\n' ' ' < "$out/failures.txt" || echo none)
  echo "$name: $sf of $expected_sf sf outputs; terra: ${terra:-no All ok line}; Cannot open lines: $cannot; failures: $failed"
  [ "$sf" = "$expected_sf" ] || status=1
  [ -n "$terra" ] || status=1
  echo "$cannot" > "$work/.cannot-$name"
done
cmp -s "$work/.cannot-ref" "$work/.cannot-ours" || { echo "the Cannot open counts differ"; status=1; }
rm -f "$work/.cannot-ref" "$work/.cannot-ours"
[ "$status" = 0 ] && echo "the two passes agree" || echo "the two passes do NOT agree"
exit "$status"
