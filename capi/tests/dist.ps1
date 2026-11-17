# Build the C library of proj-rs and install it as proj.dll and proj.lib (Windows, MSVC).
#
# Usage: capi\tests\dist.ps1 [-Release] [-Prefix <dir>]     (default prefix: target\capi-dist)
#
# Run it from a Visual Studio developer prompt: it needs `dumpbin` and `lib`. The Rust library is
# named proj_rs_capi (see capi/Cargo.toml), so the DLL is copied as proj.dll and the import
# library is made again from its exports, with proj.dll as the name to load.
param(
    [switch]$Release,
    [string]$Prefix = ""
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ($Prefix -eq "") { $Prefix = Join-Path $root "target\capi-dist" }
$profileDir = if ($Release) { "release" } else { "debug" }
$cargoArgs = @("build", "-p", "proj-rs-capi")
if ($Release) { $cargoArgs += "--release" }
if ($env:PROJ_RS_CAPI_FEATURES) { $cargoArgs += @("--features", $env:PROJ_RS_CAPI_FEATURES) }

Push-Location $root
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$built = Join-Path $targetDir $profileDir
New-Item -ItemType Directory -Force -Path (Join-Path $Prefix "lib"), (Join-Path $Prefix "bin"), (Join-Path $Prefix "include") | Out-Null

$dll = Join-Path $Prefix "bin\proj.dll"
Copy-Item (Join-Path $built "proj_rs_capi.dll") $dll -Force

# Names of the export table: ordinal, hint, RVA, name.
$names = @()
foreach ($line in (& dumpbin /exports $dll)) {
    if ($line -match '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]{8}\s+(\S+)') { $names += $Matches[1] }
}
if ($names.Count -lt 292) { throw "proj.dll exports $($names.Count) names, expected at least 292" }
$def = Join-Path $Prefix "lib\proj.def"
$lines = @("LIBRARY proj.dll", "EXPORTS")
foreach ($name in $names) {
    if ($name -eq "pj_release") { $lines += "    $name DATA" } else { $lines += "    $name" }
}
Set-Content -Path $def -Value $lines -Encoding ascii
$machine = if ($env:VSCMD_ARG_TGT_ARCH) { $env:VSCMD_ARG_TGT_ARCH } else { "x64" }
& lib /nologo "/def:$def" "/out:$(Join-Path $Prefix 'lib\proj.lib')" "/machine:$machine"
if ($LASTEXITCODE -ne 0) { throw "lib failed" }

Copy-Item (Join-Path $root "capi\include\*.h") (Join-Path $Prefix "include") -Force
Write-Output "installed in $Prefix"
