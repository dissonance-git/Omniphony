#requires -Version 5.1
#requires -RunAsAdministrator

[CmdletBinding()]
param([string]$EqualizerApoPath = '')

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$beginMarker = '# BEGIN OMNIPHONY PERSONAL BOOTSTRAP'
$endMarker = '# END OMNIPHONY PERSONAL BOOTSTRAP'
$legacyBeginMarker = '# BEGIN OMNIPHONY PERSONAL BOOTSTRAP (identity only)'

function Find-EqualizerApoPath {
    param([string]$ExplicitPath)
    if ($ExplicitPath) { return (Resolve-Path $ExplicitPath).Path }
    foreach ($keyPath in @('HKLM:\SOFTWARE\EqualizerAPO','HKLM:\SOFTWARE\WOW6432Node\EqualizerAPO')) {
        if (Test-Path $keyPath) {
            $installPath = (Get-ItemProperty $keyPath -ErrorAction SilentlyContinue).InstallPath
            if ($installPath -and (Test-Path $installPath)) { return (Resolve-Path $installPath).Path }
        }
    }
    $fallback = Join-Path $env:ProgramFiles 'EqualizerAPO'
    if (Test-Path $fallback) { return (Resolve-Path $fallback).Path }
    throw 'Equalizer APO was not found. Install/repair Equalizer APO first, then rerun this script.'
}

function Remove-MarkedBlocks {
    param([string]$Text)
    foreach ($begin in @($beginMarker, $legacyBeginMarker)) {
        $pattern = '(?ms)^' + [regex]::Escape($begin) + '\r?\n.*?^' + [regex]::Escape($endMarker) + '\r?\n?'
        $Text = [regex]::Replace($Text, $pattern, '')
    }
    return $Text
}

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$bridgeSource = Join-Path $scriptRoot 'OmniphonyVstBridge.dll'
$runtimeSource = Join-Path $scriptRoot 'omniphony_realtime.dll'
foreach ($required in @($bridgeSource, $runtimeSource)) {
    if (!(Test-Path $required)) { throw "Bootstrap artifact is incomplete: missing $required" }
}

$eapoRoot = Find-EqualizerApoPath $EqualizerApoPath
$configDir = Join-Path $eapoRoot 'config'
$configPath = Join-Path $configDir 'config.txt'
if (!(Test-Path $configPath)) { throw "Equalizer APO config.txt was not found at $configPath" }

$installDir = Join-Path $env:ProgramFiles 'Omniphony\PersonalBootstrap'
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
Copy-Item $bridgeSource (Join-Path $installDir 'OmniphonyVstBridge.dll') -Force
Copy-Item $runtimeSource (Join-Path $installDir 'omniphony_realtime.dll') -Force

$snippetPath = Join-Path $configDir 'omniphony-personal-bootstrap.txt'
$bridgeInstalled = Join-Path $installDir 'OmniphonyVstBridge.dll'
@"
# Omniphony personal bootstrap: retained Current model
VSTPlugin: Library "$bridgeInstalled"
"@ | Set-Content -Path $snippetPath -Encoding UTF8

$configText = Get-Content -Raw -Path $configPath
$cleanText = (Remove-MarkedBlocks $configText).TrimEnd("`r", "`n")
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$backupPath = "$configPath.omniphony-backup-$timestamp"
Copy-Item $configPath $backupPath -Force
$updated = @"
$cleanText

$beginMarker
Include: omniphony-personal-bootstrap.txt
$endMarker
"@
Set-Content -Path $configPath -Value $updated -Encoding UTF8

Write-Host ''
Write-Host 'Omniphony Current-model personal bootstrap installed.' -ForegroundColor Green
Write-Host "Equalizer APO: $eapoRoot"
Write-Host "Bridge:        $bridgeInstalled"
Write-Host "Config backup: $backupPath"
Write-Host ''
Write-Host 'This build is AUDIBLE: it activates the retained Omniphony Current model.'
Write-Host 'If your FiiO endpoint was already enabled in Equalizer APO/HeSuVi, leave that endpoint association alone.'
Write-Host 'If it was not, enable the real FiiO render endpoint with Equalizer APO Configurator and restart audio as requested.'
Write-Host 'Rollback at any time with uninstall-personal-bootstrap.ps1.'
