proj-rs @VERSION@ - command-line tools, @TARGET@
===============================================

The command-line tools of PROJ 9.9.0, ported to Rust: cs2cs, cct, proj, invproj, geod,
invgeod, gie and projinfo. There is one executable, bin\proj-rs.exe; the program runs the
tool whose name it is started under, or the tool named by its first argument. No library
to install, no proj.db file: the PROJ database is inside the executable.

The program needs the Visual C++ runtime (vcruntime140.dll), as any Rust program built
with Visual Studio: it is on most machines already, or install "Microsoft Visual C++
Redistributable 2015-2022" (x64).

Install
-------

1. Extract the archive where you want the tools to live, and add its bin\ folder to PATH.
2. The folder holds one small launcher per tool: cs2cs.cmd runs `proj-rs.exe cs2cs %*`.
   A launcher is not a program: another program that starts `cs2cs` without a shell
   (CreateProcess, subprocess.run(["cs2cs"])) does not find it, Ctrl+C asks "Terminate
   batch job?", and cmd expands %VAR% and ^ in the arguments. To get one real cs2cs.exe,
   proj.exe, ... per tool at no cost in disk space (NTFS hard links to proj-rs.exe), run
   once, in this folder:

     powershell -ExecutionPolicy Bypass -File link-tools.ps1

   Copying the folder with Explorer afterwards makes a full copy of each link: copy
   bin\proj-rs.exe alone and run link-tools.ps1 again at the destination.

Use
---

  echo 2.2945 48.8584 | cs2cs -d 3 +proj=longlat +datum=WGS84 +to EPSG:2154
  proj-rs cs2cs -d 3 EPSG:4326 EPSG:2154     :: the same program, by its first argument
  proj-rs --list                             :: the tool names
  proj-rs --version

As in PROJ, EPSG:4326 reads latitude first on the command line; a +proj=longlat string
reads longitude first.

Grids
-----

EPSG codes and the operation catalogue need no file. A grid (.tif, .gsb, .gtx, ...) is a
file the program looks for, as PROJ does. Put your grids in one of these places:

  - the user directory %LOCALAPPDATA%\proj; searched first. PROJ_USER_WRITABLE_DIRECTORY
    replaces it.
  - a directory named by PROJ_DATA (a list separated by ';'; PROJ_LIB is the old name).
    When it is set, the next place is not searched.
  - share\proj next to the bin\ folder: for C:\tools\bin\cs2cs.exe, C:\tools\share\proj.
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
NKG registries (copies of the licences in data\licenses\).

Source and issues: https://github.com/RandazzoMaxime/proj-rs
Checksums of the archives: SHA256SUMS, on the page of the release.
