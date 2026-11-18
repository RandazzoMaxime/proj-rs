# Replace the .cmd launchers of bin\ by NTFS hard links to proj-rs.exe: one real cs2cs.exe,
# proj.exe, ... per tool, no extra disk space. Run it once, in the folder of the archive:
#   powershell -ExecutionPolicy Bypass -File link-tools.ps1
# A tool whose link cannot be made (a file system other than NTFS, another volume) keeps its
# launcher.
$ErrorActionPreference = "Stop"
$bin = Join-Path $PSScriptRoot "bin"
$exe = Join-Path $bin "proj-rs.exe"
$tools = @(& $exe --list)
if ($LASTEXITCODE -ne 0 -or $tools.Count -lt 8) { throw "proj-rs.exe --list failed" }
$linked = 0
foreach ($tool in $tools) {
    $link = Join-Path $bin "$tool.exe"
    $launcher = Join-Path $bin "$tool.cmd"
    Remove-Item $link -Force -ErrorAction SilentlyContinue
    try {
        New-Item -ItemType HardLink -Path $link -Target $exe | Out-Null
        Remove-Item $launcher -Force -ErrorAction SilentlyContinue
        $linked++
    } catch {
        Write-Warning "no hard link for $tool, the launcher stays: $($_.Exception.Message)"
    }
}
Write-Output "$linked of $($tools.Count) tools are hard links in $bin"
