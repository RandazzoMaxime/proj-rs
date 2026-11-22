# Smoke test of the two archives of Windows x86_64, as a user would use them: unpack each in an
# empty folder, with no PROJ_DATA and no grid anywhere, run the tools through their launchers, as
# `proj-rs.exe <tool>` and, after link-tools.ps1, as real executables; compile and run a C program
# against the library with `cl`. tools/release-smoke.sh does Linux and macOS.
#
# Usage: tools\release-smoke.ps1 -Cli <cli.zip> -Libproj <libproj.zip>
#
# Run it from a Visual Studio developer prompt (`cl`). The expected values are the output of PROJ
# 9.9.0 itself (cs2cs, proj, invproj, cct, geod, invgeod, projinfo built from the tag by
# tools/build-reference.sh). A failing check does not stop the run; the exit status is 1 if one
# failed.
param(
    [Parameter(Mandatory = $true)][string]$Cli,
    [Parameter(Mandatory = $true)][string]$Libproj
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("proj-rs-smoke-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $work, (Join-Path $work "home") | Out-Null
Expand-Archive -Path $Cli -DestinationPath (Join-Path $work "cli")
Expand-Archive -Path $Libproj -DestinationPath (Join-Path $work "lib")
$cliDir = (Get-ChildItem (Join-Path $work "cli") -Directory | Select-Object -First 1).FullName
$libDir = (Get-ChildItem (Join-Path $work "lib") -Directory | Select-Object -First 1).FullName

# No data directory, no user grid: the catalogue must come from the executable.
foreach ($name in "PROJ_DATA", "PROJ_LIB", "PROJ_NETWORK", "PROJ_USER_WRITABLE_DIRECTORY") {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
}
$env:PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY = "YES"
$env:LOCALAPPDATA = Join-Path $work "home"

$script:failures = 0
function Normalize($lines) { (@($lines) -join "`n").Trim() }
function Check([string]$label, [string]$expected, $actual) {
    $actual = Normalize $actual
    if ($expected -ceq $actual) {
        Write-Output "ok   $label"
    } else {
        Write-Output "FAIL $label"
        Write-Output "  expected: $expected"
        Write-Output "  actual:   $actual"
        $script:failures++
    }
}
function Require([string]$label, [bool]$condition) { Check $label "True" "$condition" }

$tab = "`t"
$bin = Join-Path $cliDir "bin"

Write-Output "== command-line tools: $(Split-Path $cliDir -Leaf)"
foreach ($file in "README.txt", "LICENSE", "NOTICE.md", "link-tools.ps1", "data\COPYING", "data\licenses\Apache-2.0-Esri.txt", "data\licenses\CC-BY-4.0-NKG.txt", "bin\proj-rs.exe") {
    $path = Join-Path $cliDir $file
    Require "file $file" ((Test-Path $path) -and (Get-Item $path).Length -gt 0)
}
$tools = @(& (Join-Path $bin "proj-rs.exe") --list)
Check "proj-rs --list" "cct`ncs2cs`ngeod`ngie`ninvgeod`ninvproj`nproj`nprojinfo" $tools
foreach ($tool in $tools) { Require "bin\$tool.cmd" (Test-Path (Join-Path $bin "$tool.cmd")) }
Check "proj-rs --version" "proj-rs: Rel. 9.9.0, September 15th, 2026" (& (Join-Path $bin "proj-rs.exe") --version)
Check "projinfo --version" "projinfo: Rel. 9.9.0, September 15th, 2026" (& (Join-Path $bin "projinfo.cmd") --version)
Check "cct --version" "cct: Rel. 9.9.0, September 15th, 2026" (& (Join-Path $bin "cct.cmd") --version)

function Invoke-Tool([string]$exe, [string[]]$toolArgs, [string]$stdin) {
    if ($stdin -ne "") { $stdin | & $exe @toolArgs } else { & $exe @toolArgs }
}

# Each case runs the tool named by `$how`: a script block that takes the tool name and returns the
# program to start and the arguments that come before the tool's own.
function Run-Cases([string]$label, [scriptblock]$how) {
    function Run([string]$name, [string[]]$toolArgs, [string]$stdin) {
        $target = & $how $name
        Invoke-Tool $target.Exe ($target.Prefix + $toolArgs) $stdin
    }
    Check "$label cs2cs EPSG:4326 -> EPSG:2154" "648237.302${tab}6862271.682 0.000" (Run cs2cs @("-d", "3", "EPSG:4326", "EPSG:2154") "48.8584 2.2945")
    Check "$label cs2cs longlat -> EPSG:2154" "648237.302${tab}6862271.682 0.000" (Run cs2cs @("-d", "3", "+proj=longlat", "+datum=WGS84", "+to", "EPSG:2154") "2.2945 48.8584")
    Check "$label proj utm 31" "426857.99${tab}5427937.52" (Run proj @("+proj=utm", "+zone=31", "+ellps=WGS84") "2 49")
    Check "$label invproj utm 31" "3.000000${tab}45.153477" (Run invproj @("+proj=utm", "+zone=31", "+ellps=WGS84", "-f", "%.6f") "500000 5000000")
    Check "$label cct utm 31" "426857.9877   5427937.5233        0.0000        0.0000" (Run cct @("+proj=utm", "+zone=31", "+ellps=GRS80") "2 49 0 0")
    Check "$label geod forward" "49.632${tab}2.979${tab}-134.258" (Run geod @("+ellps=WGS84", "-f", "%.3f") "49 2 45 100000")
    Check "$label invgeod" "17.482${tab}-161.752${tab}233720.037" (Run invgeod @("+ellps=WGS84", "-f", "%.3f") "49 2 51 3")
    Check "$label projinfo EPSG:32631" "+proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs" (Run projinfo @("-q", "-o", "PROJ", "EPSG:32631") "")
    Check "$label projinfo EPSG:2154" "+proj=lcc +lat_0=46.5 +lon_0=3 +lat_1=49 +lat_2=44 +x_0=700000 +y_0=6600000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs" (Run projinfo @("-q", "-o", "PROJ", "EPSG:2154") "")
}
Run-Cases "launcher .cmd:" { param($name) @{ Exe = (Join-Path $bin "$name.cmd"); Prefix = @() } }
Run-Cases "proj-rs.exe <tool>:" { param($name) @{ Exe = (Join-Path $bin "proj-rs.exe"); Prefix = @($name) } }

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $cliDir "link-tools.ps1")
Check "link-tools.ps1 status" "0" "$LASTEXITCODE"
foreach ($tool in $tools) {
    Require "bin\$tool.exe exists after link-tools.ps1" (Test-Path (Join-Path $bin "$tool.exe"))
    Require "bin\$tool.cmd is gone after link-tools.ps1" (-not (Test-Path (Join-Path $bin "$tool.cmd")))
}
Run-Cases "hard link .exe:" { param($name) @{ Exe = (Join-Path $bin "$name.exe"); Prefix = @() } }

Write-Output "== C library: $(Split-Path $libDir -Leaf)"
foreach ($file in "README.txt", "LICENSE", "NOTICE.md", "data\COPYING", "bin\proj.dll", "lib\proj.lib", "lib\pkgconfig\proj.pc", "lib\cmake\proj\proj-config.cmake", "lib\cmake\proj\proj-config-version.cmake", "include\proj.h", "include\proj_constants.h", "include\proj_experimental.h", "include\proj_symbol_rename.h") {
    $path = Join-Path $libDir $file
    Require "file $file" ((Test-Path $path) -and (Get-Item $path).Length -gt 0)
}
foreach ($header in "proj.h", "proj_constants.h", "proj_experimental.h", "proj_symbol_rename.h", "geodesic.h") {
    Check "include\$header is the header of the repository" (Get-FileHash (Join-Path $root "capi\include\$header")).Hash (Get-FileHash (Join-Path $libDir "include\$header")).Hash
}

$cdir = Join-Path $work "c"
New-Item -ItemType Directory -Force -Path $cdir | Out-Null
Push-Location $cdir
try {
    & cl /nologo /W3 /I (Join-Path $libDir "include") (Join-Path $root "tools\release\smoke.c") /Fe:smoke.exe /link (Join-Path $libDir "lib\proj.lib")
    Check "cl status" "0" "$LASTEXITCODE"
    $env:PATH = (Join-Path $libDir "bin") + ";" + $env:PATH
    $expectedC = "version 9.9.0`nlambert93 648237.302 6862271.682`ninverse 48.858400 2.294500`nutm31 426857.988 5427937.523"
    Check "C program linked to proj.lib, run with proj.dll" $expectedC (& .\smoke.exe)

    # CMake finds the library as it finds PROJ 9.9.0, from a copy of the archive moved elsewhere.
    $moved = Join-Path $work "moved-lib"
    Copy-Item $libDir $moved -Recurse
    $env:PATH = (Join-Path $moved "bin") + ";" + $env:PATH
    $cmakeDir = Join-Path $work "cmake"
    $log = & cmake -S (Join-Path $root "tools\release\cmake-smoke") -B $cmakeDir "-DCMAKE_PREFIX_PATH=$moved" 2>&1
    Check "cmake configure status" "0" "$LASTEXITCODE"
    Check "find_package(PROJ 9.9 CONFIG) reports the version" "-- PROJ_VERSION=9.9.0" (($log | Select-String -SimpleMatch "-- PROJ_VERSION=" | Select-Object -First 1).Line)
    & cmake --build $cmakeDir --config Release | Out-Null
    Check "cmake build status" "0" "$LASTEXITCODE"
    Check "C program built with CMake, PROJ::proj" $expectedC (& (Join-Path $cmakeDir "Release\smoke.exe"))
    $pc = Get-Content -Raw (Join-Path $moved "lib\pkgconfig\proj.pc")
    Check "proj.pc names version 9.9.0" "True" "$($pc -match '(?m)^Version: 9\.9\.0\s*$')"
    Check "proj.pc and the CMake files have no placeholder left" "0" "$((Get-Content (Join-Path $moved 'lib\pkgconfig\proj.pc'), (Join-Path $moved 'lib\cmake\proj\*.cmake') | Select-String -Pattern '@[A-Z_]*@').Count)"
} finally {
    Pop-Location
}

Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
if ($script:failures -ne 0) {
    Write-Error "$($script:failures) check(s) failed"
    exit 1
}
Write-Output "all checks passed"
