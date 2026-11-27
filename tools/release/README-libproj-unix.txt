proj-rs @VERSION@ - C library, a replacement for libproj 9.9.0, @TARGET@
=====================================================================

The proj.h of PROJ 9.9.0 on the proj-rs engine: the same header, the same 292 function
names and signatures, the same structures. A program written for libproj links against it
without a change. The PROJ database is inside the library; no proj.db file is needed.

Contents
--------

  lib/@LIBFILES@
  lib/pkgconfig/proj.pc           for pkg-config, as PROJ installs it
  share/proj/proj.db              the database, also inside the library (see below)
  lib/cmake/proj/proj-config.cmake, proj-config-version.cmake
                                  for CMake's find_package(PROJ), target PROJ::proj
  include/proj.h, proj_constants.h, proj_experimental.h, proj_symbol_rename.h
                                  (the headers of PROJ 9.9.0, byte for byte)

The library has the name and the version of libproj 9.9.0 (@SONAME@), so a program built
against the PROJ library finds it. pkg-config and CMake report the version of PROJ whose
proj.h it implements, 9.9.0. Both files follow the folder they are in: the archive works
wherever it is unpacked, and after a copy to /usr/local.

Use
---

With pkg-config:

  export PKG_CONFIG_PATH=$PWD/lib/pkgconfig
  cc demo.c $(pkg-config --cflags --libs proj) -Wl,-rpath,$PWD/lib

With CMake, in the CMakeLists.txt of the project:

  find_package(PROJ 9 CONFIG REQUIRED)
  target_link_libraries(my_program PRIVATE PROJ::proj)

and on the command line: cmake -DCMAKE_PREFIX_PATH=$PWD ...

By hand:

  cc demo.c -Iinclude -Llib -lproj -Wl,-rpath,$PWD/lib

To link the static library instead:

  cc demo.c -Iinclude lib/libproj.a @STATICLIBS@

To install it system-wide (cp -a keeps the symbolic links):

  cp -a lib/. /usr/local/lib/ && cp include/*.h /usr/local/include/@LDCONFIG@

lib/. holds pkgconfig/ and cmake/proj/, so pkg-config and CMake find it there too.

Calls that differ from PROJ, if any, are listed in docs/API-v1.md of the source tree
(https://github.com/RandazzoMaxime/proj-rs). A pair of CRS made by proj_create_crs_to_crs
transforms in the axis order of its CRS, as in PROJ: EPSG:4326 is latitude, longitude until
proj_normalize_for_visualization.

The database
------------

The PROJ database is inside the library: EPSG codes and the operation catalogue need no file.
share/proj/proj.db is the same database, byte for byte, for programs that check that the file
exists before they call PROJ, as pyproj does: point PROJ_DATA at that folder for them
(PROJ_DATA=$PWD/share/proj). The library itself does not need it.

Grids
-----

EPSG codes and the operation catalogue need no file. A grid (.tif, .gsb, .gtx, ...) is a
file the library looks for, as PROJ does. Put your grids in one of these places:

  - the user directory: @USERDIR@; searched first.
    PROJ_USER_WRITABLE_DIRECTORY replaces it.
  - a directory named by PROJ_DATA (a list separated by ':'; PROJ_LIB is the old name).
    When it is set, the next place is not searched.
  - share/proj next to the folder of the executable.
  - the working directory, last.

proj_context_set_search_paths replaces all of the above, as in PROJ. Grids are published
at https://cdn.proj.org.

This build does not download grids: the network is not compiled in (no TLS library), as a
PROJ built without libcurl. proj_context_set_enable_network(ctx, 1) fetches nothing unless
the application gives its own client with proj_context_set_network_callbacks. A grid that
is missing is an error.

Licence
-------

proj-rs is under the GNU AGPL v3.0 (LICENSE; a commercial license is on request: https://github.com/helix-aero/proj-rs/blob/main/COMMERCIAL-LICENSE.md). NOTICE.md lists the other works it contains and
their terms, among them the EPSG Geodetic Parameter Dataset, whose Terms of Use apply to
whoever passes this software on (https://epsg.org/terms-of-use.html), and the ESRI, IGNF and
NKG registries (copies of the licences in data/licenses/).

Source and issues: https://github.com/RandazzoMaxime/proj-rs
Checksums of the archives: SHA256SUMS, on the page of the release.
