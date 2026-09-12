# DSH Dock - Windows read-only probe: does a NON-global pnpm install require
# symlink privilege?  (2026-09-11, option B feasibility)
#
# WHY: pnpm's GLOBAL install creates a "global package hash link" (a symlink) at
#   <engines>\global\v11\<hash>\<hash>
# On a normal Windows account (no Developer Mode / not elevated) that fails with
#   os error 5 (access denied)  -> engine bootstrap can never install dsh.
# That mechanism is NOT avoidable via config (7 candidates were exhausted).
#
# Option B = stop using `pnpm add -g`; install dsh as a plain project dependency
# under <engines>\dsh-runtime\ (nodeLinker: hoisted = real directories) and ship
# our own shim in <engines>\bin. Verified end-to-end on macOS; the ONLY fact that
# cannot be checked off-Windows is how pnpm creates node_modules\.bin entries.
#
# THIS SCRIPT IS READ-ONLY WITH RESPECT TO THE APP:
#   - never touches <engines>\, never writes the registry, never installs globally
#   - works only inside a fresh %TEMP% directory, which it deletes at the end
#   - MUST be run in a NON-elevated PowerShell: the normal-account case is the point
#
# Usage (in a normal PowerShell window):
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\probe-windows-engine-install.ps1
#
# Output is intentionally ASCII-only so no console/encoding setup can corrupt it.

$ErrorActionPreference = 'Continue'

$App  = Join-Path $env:APPDATA 'io.github.realguan.dsh-dock'
$Pnpm = Join-Path $App 'engines\bin\pnpm.exe'
$Node = Join-Path $App 'engines\bin\node.exe'

function Section($t) { Write-Host "`n===== $t =====" -ForegroundColor Cyan }

Section '0. Environment facts'
$devMode = (Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' `
            -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
Write-Host ("DeveloperMode AllowDevelopmentWithoutDevLicense = {0}   (1=enabled, 0/blank=disabled)" -f $devMode)
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
             [Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host ("IsAdministrator = {0}   (expected False; if True, re-run in a normal window)" -f $isAdmin)
Write-Host ("pnpm.exe present = {0}" -f (Test-Path $Pnpm))
Write-Host ("node.exe present = {0}" -f (Test-Path $Node))

if (-not (Test-Path $Pnpm)) {
    Write-Host "`n[ABORT] pnpm.exe not found at $Pnpm" -ForegroundColor Red
    Write-Host "The app has not staged its engine yet. Open the app once so it installs pnpm, then re-run." -ForegroundColor Yellow
    return
}
if ($isAdmin) {
    Write-Host "`n[WARN] Running elevated: the result will NOT represent a normal account." -ForegroundColor Yellow
}

Section '1. Create a temp project (inside %TEMP% only)'
$Tmp   = Join-Path $env:TEMP ('dsh-b-probe-' + (Get-Random))
$Proj  = Join-Path $Tmp 'proj'
$Home2 = Join-Path $Tmp 'home'
New-Item -ItemType Directory -Force -Path $Proj, $Home2 | Out-Null
Set-Content -Path (Join-Path $Proj 'pnpm-workspace.yaml') -Value 'nodeLinker: hoisted' -Encoding ascii
$env:PNPM_HOME = Join-Path $Tmp 'pnpmhome'
Write-Host "temp dir  = $Tmp"
Write-Host "PNPM_HOME = $env:PNPM_HOME"

Section '2. Non-global install of a small bin-bearing package (DECISIVE STEP)'
Push-Location $Proj
& $Pnpm add semver --registry=https://registry.npmjs.org 2>&1 | ForEach-Object { Write-Host "  $_" }
$code = $LASTEXITCODE
Pop-Location
Write-Host ""
Write-Host ("pnpm add exit code = {0}" -f $code) -ForegroundColor $(if ($code -eq 0) { 'Green' } else { 'Red' })
Write-Host "  exit 0  -> a non-global install SUCCEEDS on a normal account => option B is viable"
Write-Host "  non-0   -> read the error text above; if it is os error 5, the failing step is .bin linking"

Section '3a. Was global\ created?  (touching the hash-link mechanism)'
if (Test-Path (Join-Path $env:PNPM_HOME 'global')) {
    Write-Host "  [!!] global\ WAS created - the global install mechanism was still touched" -ForegroundColor Yellow
} else {
    Write-Host "  [OK] global\ NOT created - hash-link mechanism untouched" -ForegroundColor Green
}

Section '3b. Form of node_modules\.bin entries  (shim file vs symlink)'
$binDir = Join-Path $Proj 'node_modules\.bin'
if (Test-Path $binDir) {
    Get-ChildItem $binDir -Force | Sort-Object Name | ForEach-Object {
        $isLink = ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
        $kind = if ($isLink) { 'SYMLINK/JUNCTION (needs privilege on Windows)' } else { 'plain file (cmd shim - no privilege needed)' }
        Write-Host ("  {0,-28} {1}" -f $_.Name, $kind)
    }
} else {
    Write-Host "  (no .bin directory)"
}

Section '3c. Any symlink/junction elsewhere in the tree (excluding .bin)?'
$links = Get-ChildItem $Proj -Recurse -Force -ErrorAction SilentlyContinue | Where-Object {
    ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -and $_.FullName -notlike '*\.bin\*'
}
if ($links) {
    $links | ForEach-Object { Write-Host ("  [!!] " + $_.FullName) }
} else {
    Write-Host "  [OK] zero links outside .bin (the layout option B needs)" -ForegroundColor Green
}

Section '4. Does a self-authored shim run?  (simulates <engines>\bin\dsh.cmd)'
$entry = Join-Path $Proj 'node_modules\semver\bin\semver.js'
if ((Test-Path $Node) -and (Test-Path $entry)) {
    $shim = Join-Path $Tmp 'semver-shim.cmd'
    Set-Content -Path $shim -Value "@echo off`r`n`"$Node`" `"$entry`" %*" -Encoding ascii
    $shimIsLink = ((Get-Item $shim).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
    Write-Host ("shim is a plain file (not a link) = {0}" -f (-not $shimIsLink))
    & $shim --version 2>&1 | ForEach-Object { Write-Host ("  semver reports = {0}" -f $_) }
    Write-Host "  (in the real fix this shim is <engines>\bin\dsh.cmd and targets"
    Write-Host "   <engines>\dsh-runtime\node_modules\@deepseek-ai\dsh\lib\bin.js)"
} else {
    Write-Host "  (skipped: node.exe or the package entry point not found)"
}

Section '5. Cleanup'
Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
Write-Host "removed $Tmp"

Write-Host "`nPlease paste the whole output back to the Lead (sections 2 and 3 matter most)." -ForegroundColor Green
