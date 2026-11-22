# Build and pack the two archives of Windows x86_64 (MSVC). tools/release-pack.sh does Linux and macOS.
#
# Usage: tools\release-pack.ps1 -Target windows-x86_64 [-Out <dir>]   (default: target\release-archives)
#
# Makes, with <version> read from Cargo.toml:
#   proj-rs-<version>-<Target>-cli.zip      bin\proj-rs.exe, one .cmd launcher per tool, link-tools.ps1
#   proj-rs-<version>-<Target>-libproj.zip  bin\proj.dll, lib\proj.lib, include\*.h,
#                                           lib\pkgconfig\proj.pc, lib\cmake\proj\*.cmake
# Each holds LICENSE, NOTICE.md (with the licence texts it points to) and a README.txt.
#
# The archive carries launchers, not hard links: a zip stores each link as a full copy (8 times
# the size). link-tools.ps1, in the archive, makes the hard links where the archive is unpacked.
# Run it from a Visual Studio developer prompt (dist.ps1 needs `dumpbin` and `lib`), then
# tools\release-smoke.ps1.
param(
    [Parameter(Mandatory = $true)][string]$Target,
    [string]$Out = ""
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($Out -eq "") { $Out = Join-Path $root "target\release-archives" }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$version = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("proj-rs-release-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $work | Out-Null

try {
    $cliName = "proj-rs-$version-$Target-cli"
    $libName = "proj-rs-$version-$Target-libproj"

    & (Join-Path $root "tools\dist-cli.ps1") -Release -Prefix (Join-Path $work "cli-prefix") -Launchers
    & (Join-Path $root "capi\tests\dist.ps1") -Release -Prefix (Join-Path $work "lib-prefix")

    function Add-Licences([string]$dir) {
        New-Item -ItemType Directory -Force -Path (Join-Path $dir "data\licenses") | Out-Null
        Copy-Item (Join-Path $root "LICENSE"), (Join-Path $root "NOTICE.md") $dir
        Copy-Item (Join-Path $root "data\COPYING") (Join-Path $dir "data")
        Copy-Item (Join-Path $root "data\licenses\*") (Join-Path $dir "data\licenses")
    }
    function Add-Readme([string]$template, [string]$dest) {
        $text = (Get-Content -Raw -Path $template).Replace("@VERSION@", $version).Replace("@TARGET@", $Target)
        Set-Content -Path $dest -Value $text -Encoding ascii -NoNewline
    }
    function Pack([string]$name) {
        $zip = Join-Path $Out "$name.zip"
        Remove-Item $zip -Force -ErrorAction SilentlyContinue
        Compress-Archive -Path (Join-Path $work $name) -DestinationPath $zip -CompressionLevel Optimal
        Get-Item $zip | Format-Table Name, Length
    }

    $cli = Join-Path $work $cliName
    New-Item -ItemType Directory -Force -Path $cli | Out-Null
    Copy-Item (Join-Path $work "cli-prefix\bin") (Join-Path $cli "bin") -Recurse
    Copy-Item (Join-Path $root "tools\release\link-tools.ps1") $cli
    Add-Licences $cli
    Add-Readme (Join-Path $root "tools\release\README-cli-windows.txt") (Join-Path $cli "README.txt")
    Pack $cliName

    $lib = Join-Path $work $libName
    New-Item -ItemType Directory -Force -Path $lib | Out-Null
    Copy-Item (Join-Path $work "lib-prefix\bin"), (Join-Path $work "lib-prefix\lib"), (Join-Path $work "lib-prefix\include") $lib -Recurse
    # pkg-config and CMake find the library as they find PROJ: lib\pkgconfig\proj.pc, lib\cmake\proj.
    $header = Get-Content -Raw (Join-Path $root "capi\include\proj.h")
    $part = { param($name) [regex]::Match($header, "#define PROJ_VERSION_$name (\d+)").Groups[1].Value }
    $major = & $part "MAJOR"; $minor = & $part "MINOR"; $patch = & $part "PATCH"
    if (-not $major -or -not $minor -or -not $patch) { throw "no PROJ_VERSION_* in proj.h" }
    $values = @{
        "@VERSION@" = $version; "@PROJ_VERSION@" = "$major.$minor.$patch"
        "@PROJ_VERSION_MAJOR@" = $major; "@PROJ_VERSION_MINOR@" = $minor; "@PROJ_VERSION_PATCH@" = $patch
        "@STATICLIBS@" = ""; "@IMPORTED_LOCATION@" = "bin/proj.dll"
        "@IMPORTED_KEY@" = "IMPORTED_IMPLIB_RELEASE"; "@IMPORTED_VALUE@" = '${_PROJ_ROOT}/lib/proj.lib'
    }
    function Fill([string]$template, [string]$dest) {
        $text = Get-Content -Raw -Path $template
        foreach ($key in $values.Keys) { $text = $text.Replace($key, $values[$key]) }
        New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
        Set-Content -Path $dest -Value $text -Encoding ascii -NoNewline
    }
    Fill (Join-Path $root "tools\release\proj.pc.in") (Join-Path $lib "lib\pkgconfig\proj.pc")
    Fill (Join-Path $root "tools\release\cmake\proj-config.cmake.in") (Join-Path $lib "lib\cmake\proj\proj-config.cmake")
    Fill (Join-Path $root "tools\release\cmake\proj-config-version.cmake.in") (Join-Path $lib "lib\cmake\proj\proj-config-version.cmake")
    Add-Licences $lib
    Add-Readme (Join-Path $root "tools\release\README-libproj-windows.txt") (Join-Path $lib "README.txt")
    Pack $libName
} finally {
    Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
