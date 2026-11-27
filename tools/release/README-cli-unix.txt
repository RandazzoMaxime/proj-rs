proj-rs @VERSION@ - command-line tools, @TARGET@
===============================================

The command-line tools of PROJ 9.9.0, ported to Rust: cs2cs, cct, proj, invproj, geod,
invgeod, gie and projinfo. There is one executable, bin/proj-rs; every other name in bin/
is a symbolic link to it, and the program runs the tool it is started under. No library to
install, no proj.db file: the PROJ database is inside the executable.

Install
-------

  Keep the links together with the executable (cp -a keeps symbolic links):

    cp -a bin/. /usr/local/bin/              # or any directory of your PATH

  or leave the folder where it is and add its bin/ directory to PATH. The links are
  relative, so the folder can be moved as a whole.

Use
---

  echo "2.2945 48.8584" | cs2cs -d 3 +proj=longlat +datum=WGS84 +to EPSG:2154
  proj-rs cs2cs -d 3 EPSG:4326 EPSG:2154     # the same program, by its first argument
  proj-rs --list                             # the tool names
  proj-rs --version

As in PROJ, EPSG:4326 reads latitude first on the command line; a +proj=longlat string
reads longitude first.

Grids
-----

EPSG codes and the operation catalogue need no file. A grid (.tif, .gsb, .gtx, ...) is a
file the program looks for, as PROJ does. Put your grids in one of these places:

  - the user directory: $XDG_DATA_HOME/proj or ~/.local/share/proj (Linux),
    ~/Library/Application Support/proj (macOS); searched first. PROJ_USER_WRITABLE_DIRECTORY
    replaces it.
  - a directory named by PROJ_DATA (a list separated by ':'; PROJ_LIB is the old name).
    When it is set, the next place is not searched.
  - share/proj next to the folder of the executable: for /opt/tools/bin/cs2cs,
    /opt/tools/share/proj.
  - the working directory, last.

`projinfo --searchpaths` prints the directories of the current run. Grids are published
at https://cdn.proj.org.

This build does not download grids: the network is not compiled in (no TLS library), as a
PROJ built without libcurl. A grid that is missing is an error.

Licence
-------

proj-rs is under the GNU AGPL v3.0 (LICENSE; a commercial license is on request: https://github.com/helix-aero/proj-rs/blob/main/COMMERCIAL-LICENSE.md). NOTICE.md lists the other works it contains and
their terms, among them the EPSG Geodetic Parameter Dataset, whose Terms of Use apply to
whoever passes this software on (https://epsg.org/terms-of-use.html), and the ESRI, IGNF and
NKG registries (copies of the licences in data/licenses/).

Source and issues: https://github.com/RandazzoMaxime/proj-rs
Checksums of the archives: SHA256SUMS, on the page of the release.
