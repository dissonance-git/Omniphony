#requires -Version 5.1
#requires -RunAsAdministrator

[CmdletBinding()]
param([string]$EqualizerApoPath = '')

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$beginMarker = '# BEGIN OMNIPHONY PERSONAL BOOTSTRAP'
$legacyBeginMarker = '# BEGIN OMNIPHONY PERSONAL BOOTSTRAP (identity only)'
$endMarker = '# END OMNIPHONY PERSONAL BOOTSTRAP'

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
    throw 'Equalizer APO was not found.'
}

function Remove-MarkedBlocks {
    param([string]$Text)
    foreach ($begin in @($beginMarker, $legacyBeginMarker)) {
        $pattern = '(?ms)^' + [regex]::Escape($begin) + '\r?\n.*?^' + [regex]::Escape($endMarker) + '\r?\n?'
        $Text = [regex]::Replace($Text, $pattern, '')
    }
    return $Text
}

$eapoRoot = Find-EqualizerApoPath $EqualizerApoPath
$configDir = Join-Path $eapoRoot 'config'
$configPath = Join-Path $configDir 'config.txt'
if (!(Test-Path $configPath)) { throw "Equalizer APO config.txt was not found at $configPath" }
$configText = Get-Content -Raw -Path $configPath
$updated = (Remove-MarkedBlocks $configText).TrimEnd("`r", "`n") + "`r`n"
Set-Content -Path $configPath -Value $updated -Encoding UTF8
$snippetPath = Join-Path $configDir 'omniphony-personal-bootstrap.txt'
if (Test-Path $snippetPath) { Remove-Item $snippetPath -Force }
$installDir = Join-Path $env:ProgramFiles 'Omniphony\PersonalBootstrap'
if (Test-Path $installDir) { Remove-Item $installDir -Recurse -Force }
Write-Host 'Omniphony personal bootstrap removed. Equalizer APO and unrelated configuration were preserved.' -ForegroundColor Green
