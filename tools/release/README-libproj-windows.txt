proj-rs @VERSION@ - C library, a replacement for proj.dll of PROJ 9.9.0, @TARGET@
=============================================================================

The proj.h of PROJ 9.9.0 on the proj-rs engine: the same header, the same 292 function
names and signatures, the same structures. A program written for libproj links against it
without a change. The PROJ database is inside the DLL; no proj.db file is needed.

The DLL needs the Visual C++ runtime (vcruntime140.dll), as any Rust library built with
Visual Studio: it is on most machines already, or install "Microsoft Visual C++
Redistributable 2015-2022" (x64).

Contents
--------

  bin\proj.dll
  lib\proj.lib, proj.def          (import library, made from the exports of proj.dll)
  lib\pkgconfig\proj.pc           for pkg-config, as PROJ installs it
  lib\cmake\proj\proj-config.cmake, proj-config-version.cmake
                                  for CMake's find_package(PROJ), target PROJ::proj
  include\proj.h, proj_constants.h, proj_experimental.h, proj_symbol_rename.h
                                  (the headers of PROJ 9.9.0, byte for byte)

pkg-config and CMake report the version of PROJ whose proj.h the library implements, 9.9.0.
Both files follow the folder they are in: the archive works wherever it is unpacked.

Use
---

With CMake, in the CMakeLists.txt of the project:

  find_package(PROJ 9 CONFIG REQUIRED)
  target_link_libraries(my_program PRIVATE PROJ::proj)

and on the command line: cmake -DCMAKE_PREFIX_PATH=<this folder> ...

By hand:

  cl /I include demo.c /link lib\proj.lib
  set PATH=%CD%\bin;%PATH%
  demo.exe

proj.dll must be next to the program or in a folder of PATH when the program runs.

Calls that differ from PROJ, if any, are listed in docs/API-v1.md of the source tree
(https://github.com/RandazzoMaxime/proj-rs). A pair of CRS made by proj_create_crs_to_crs
transforms in the axis order of its CRS, as in PROJ: EPSG:4326 is latitude, longitude until
proj_normalize_for_visualization.

Grids
-----

EPSG codes and the operation catalogue need no file. A grid (.tif, .gsb, .gtx, ...) is a
file the library looks for, as PROJ does. Put your grids in one of these places:

  - the user directory %LOCALAPPDATA%\proj; searched first.
    PROJ_USER_WRITABLE_DIRECTORY replaces it.
  - a directory named by PROJ_DATA (a list separated by ';'; PROJ_LIB is the old name).
    When it is set, the next place is not searched.
  - share\proj next to the folder of the executable.
  - the working directory, last.

proj_context_set_search_paths replaces all of the above, as in PROJ. Grids are published
at https://cdn.proj.org.

This build does not download grids: the network is not compiled in (no TLS library), as a
PROJ built without libcurl. proj_context_set_enable_network(ctx, 1) fetches nothing unless
the application gives its own client with proj_context_set_network_callbacks. A grid that
is missing is an error.

Licence
-------

proj-rs is under the MIT licence (LICENSE). NOTICE.md lists the other works it contains and
their terms, among them the EPSG Geodetic Parameter Dataset, whose Terms of Use apply to
whoever passes this software on (https://epsg.org/terms-of-use.html), and the ESRI, IGNF and
NKG registries (copies of the licences in data\licenses\).

Source and issues: https://github.com/RandazzoMaxime/proj-rs
Checksums of the archives: SHA256SUMS, on the page of the release.
