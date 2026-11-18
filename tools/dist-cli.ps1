# Build the command-line tools of proj-rs as ONE executable and install it under the names of the
# PROJ tools: PREFIX\bin\proj-rs.exe, plus cs2cs.exe, proj.exe, cct.exe, ... that are hard links
# to it (no disk space for the copies, the program runs the tool whose name it is started under).
#
# Usage: tools\dist-cli.ps1 [-Release] [-Prefix <dir>] [-Launchers]    (default prefix: target\cli-dist)
#
# $env:PROJ_RS_CLI_FEATURES = "network" adds `projsync` and the HTTPS grid download.
#
# -Release builds with the `release-cli` profile of Cargo.toml: the `release` profile plus
# `panic = "abort"`, fat LTO and stripped symbols.
#
# Where hard links are not possible (a file system other than NTFS, or a prefix on another volume
# than the build), or with -Launchers, each name is a small `name.cmd` that runs
# `proj-rs.exe name %*`. A launcher is not an executable: a program that starts `cs2cs` without a
# shell (CreateProcess, `subprocess.run(["cs2cs"])`) does not find it, Ctrl+C asks "Terminate
# batch job?", and cmd expands `%VAR%` and `^` in the arguments. Prefer the hard links. Copying the
# folder with Explorer makes a full copy of each link: copy `proj-rs.exe` alone and run this
# script again at the destination.
param(
    [switch]$Release,
    [string]$Prefix = "",
    [switch]$Launchers
)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($Prefix -eq "") { $Prefix = Join-Path $root "target\cli-dist" }
$profileDir = if ($Release) { "release-cli" } else { "debug" }
$cargoArgs = @("build", "--bin", "proj-rs")
if ($Release) { $cargoArgs += @("--profile", "release-cli") }
if ($env:PROJ_RS_CLI_FEATURES) { $cargoArgs += @("--features", $env:PROJ_RS_CLI_FEATURES) }

Push-Location $root
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
$bin = Join-Path $Prefix "bin"
New-Item -ItemType Directory -Force -Path $bin | Out-Null
$exe = Join-Path $bin "proj-rs.exe"
Copy-Item (Join-Path $targetDir "$profileDir\proj-rs.exe") $exe -Force

# The tool names come from the executable, so they follow the features it was built with.
$tools = @(& $exe --list)
if ($LASTEXITCODE -ne 0 -or $tools.Count -lt 8) { throw "proj-rs.exe --list failed" }

$mode = "hard links"
foreach ($tool in $tools) {
    $link = Join-Path $bin "$tool.exe"
    $launcher = Join-Path $bin "$tool.cmd"
    Remove-Item $link, $launcher -Force -ErrorAction SilentlyContinue
    $linked = $false
    if (-not $Launchers) {
        try {
            New-Item -ItemType HardLink -Path $link -Target $exe | Out-Null
            $linked = $true
        } catch {
            $linked = $false
        }
    }
    if (-not $linked) {
        $mode = "launchers (.cmd)"
        $lines = @("@echo off", "`"%~dp0proj-rs.exe`" $tool %*", "exit /b %ERRORLEVEL%")
        Set-Content -Path $launcher -Value $lines -Encoding ascii
    }
}
Write-Output "installed in $bin, names as $mode"
