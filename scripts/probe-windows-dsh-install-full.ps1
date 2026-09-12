# DSH Dock - Windows probe #2: FULL end-to-end dsh install (option B, ADR-0017)
#
# WHY THIS EXISTS (probe #1 is not enough):
#   Probe #1 proved the MECHANISM: a non-global `pnpm add` needs no symlink privilege,
#   `global\` is not created, and .bin entries are plain cmd shims. But it installed
#   `semver` -- a pure-JS package with no build scripts.
#
#   dsh pulls native/helper build scripts (koffi, node-pty, protobufjs, @google/genai,
#   @deepseek-ai/dsh-subprocess-local). On Windows the OLD code path never got past the
#   global hash-link failure, so THOSE steps have never run on a real Windows machine.
#   This probe closes that gap BEFORE the app is rebuilt: real install, into %TEMP%,
#   then it runs the EXACT shim the app ships and asks dsh for its version.
#
# READ-ONLY WITH RESPECT TO THE APP:
#   uses the app's bundled pnpm.exe / node.exe, but writes only under %TEMP%
#   never touches <engines>\, never writes the registry, never installs globally
#   deletes its temp directory at the end
#   RUN IT IN A NORMAL (non-elevated) POWERSHELL - the normal account is the point
#
# NETWORK: downloads the real dsh tree (tens of MB). Expect a few minutes.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\probe-windows-dsh-install-full.ps1

param(
    [string]$DshVersion = '0.1.5-rc.2',
    [string]$Registry   = 'https://registry.npmjs.org'
)

$ErrorActionPreference = 'Continue'

$App  = Join-Path $env:APPDATA 'io.github.realguan.dsh-dock'
$Pnpm = Join-Path $App 'engines\bin\pnpm.exe'
$Node = Join-Path $App 'engines\bin\node.exe'

function Section($t) { Write-Host "`n===== $t =====" -ForegroundColor Cyan }

Section '0. Environment'
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
             [Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host ("IsAdministrator = {0}   (expected False)" -f $isAdmin)
Write-Host ("pnpm.exe = {0}" -f (Test-Path $Pnpm))
Write-Host ("node.exe = {0}" -f (Test-Path $Node))
if (-not (Test-Path $Pnpm) -or -not (Test-Path $Node)) {
    Write-Host "`n[ABORT] Engine not staged yet. Open the app once, then re-run." -ForegroundColor Red
    return
}

Section '1. Temp layout (mirrors <engines>\ exactly)'
$Tmp = Join-Path $env:TEMP ('dsh-full-probe-' + (Get-Random))
$Bin = Join-Path $Tmp 'bin'
$Rt  = Join-Path $Tmp 'dsh-runtime'
New-Item -ItemType Directory -Force -Path $Bin, $Rt | Out-Null
Set-Content -Path (Join-Path $Rt 'package.json') -Value '{"name":"dsh-runtime","private":true,"version":"0.0.0"}' -Encoding ascii
Set-Content -Path (Join-Path $Rt 'pnpm-workspace.yaml') -Value 'nodeLinker: hoisted' -Encoding ascii
$env:PNPM_HOME = Join-Path $Tmp 'pnpmhome'
Write-Host "temp = $Tmp   (bin\ + dsh-runtime\ , same shape as <engines>\)"

# node.exe must sit next to the shim for the shipped `%~dp0node.exe` form to work.
# Hardlink first (no privilege needed, same volume); fall back to copy.
$NodeInBin = Join-Path $Bin 'node.exe'
try {
    New-Item -ItemType HardLink -Path $NodeInBin -Target $Node -ErrorAction Stop | Out-Null
    Write-Host "node.exe linked into bin\ (hardlink, no privilege needed)"
} catch {
    Copy-Item $Node $NodeInBin -Force
    Write-Host "node.exe copied into bin\"
}

Section "2. REAL install: pnpm add @deepseek-ai/dsh@$DshVersion  (this is what the fix runs)"
Write-Host "   registry = $Registry ; downloading, please wait..." -ForegroundColor DarkGray
Push-Location $Rt
& $Pnpm add "@deepseek-ai/dsh@$DshVersion" --registry=$Registry `
    --allow-build=@deepseek-ai/dsh-subprocess-local --allow-build=@google/genai `
    --allow-build=koffi --allow-build=node-pty --allow-build=protobufjs 2>&1 |
    ForEach-Object { Write-Host "  $_" }
$code = $LASTEXITCODE
Pop-Location
Write-Host ""
Write-Host ("install exit code = {0}" -f $code) -ForegroundColor $(if ($code -eq 0) { 'Green' } else { 'Red' })
Write-Host "  exit 0 -> the whole install chain (incl. native build scripts) works on a normal account"

Section '3. Package entry point'
$Entry = Join-Path $Rt 'node_modules\@deepseek-ai\dsh\lib\bin.js'
if (Test-Path $Entry) {
    Write-Host ("  [OK] {0} ({1} bytes)" -f $Entry, (Get-Item $Entry).Length) -ForegroundColor Green
} else {
    Write-Host "  [!!] entry MISSING - install incomplete" -ForegroundColor Red
}

Section '4. Layout checks (ADR-0017 expectations)'
if (Test-Path (Join-Path $env:PNPM_HOME 'global')) {
    Write-Host "  [!!] global\ WAS created - unexpected" -ForegroundColor Yellow
} else {
    Write-Host "  [OK] global\ not created (hash-link mechanism untouched)" -ForegroundColor Green
}
$links = Get-ChildItem $Rt -Recurse -Force -ErrorAction SilentlyContinue | Where-Object {
    ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -and $_.FullName -notlike '*\.bin\*'
}
if ($links) { $links | ForEach-Object { Write-Host ("  [!!] link: " + $_.FullName) } }
else { Write-Host "  [OK] zero links outside .bin" -ForegroundColor Green }

Section '5. DECISIVE: write the EXACT shipped shim, run it, ask dsh its version'
# Byte-for-byte the form the app writes to <engines>\bin\dsh.cmd (engines.rs::dsh_shim_windows).
$Shim = Join-Path $Bin 'dsh.cmd'
$body = "@echo off`r`n" +
        "rem DSH Dock engine launcher (ADR-0017: shell-owned shim, not the pnpm global shim).`r`n" +
        "`"%~dp0node.exe`" `"%~dp0..\dsh-runtime\node_modules\@deepseek-ai\dsh\lib\bin.js`" %*`r`n"
Set-Content -Path $Shim -Value $body -Encoding ascii -NoNewline
Write-Host "wrote $Shim"
Write-Host "  content:" -ForegroundColor DarkGray
Get-Content $Shim | ForEach-Object { Write-Host ("    " + $_) -ForegroundColor DarkGray }

Write-Host "`n  running: $Shim --version"
$ver = & $Shim --version 2>&1
$verCode = $LASTEXITCODE
$ver | ForEach-Object { Write-Host ("  dsh reports = {0}" -f $_) }
Write-Host ""
if ($verCode -eq 0 -and $ver) {
    Write-Host "  [OK] the shipped shim launches dsh on Windows" -ForegroundColor Green
} else {
    Write-Host ("  [!!] shim exit={0} - investigate the output above" -f $verCode) -ForegroundColor Red
}

Section '6. Cleanup'
Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
Write-Host "removed $Tmp"

Write-Host "`nPaste the whole output back to the Lead. Sections 2 and 5 matter most." -ForegroundColor Green
