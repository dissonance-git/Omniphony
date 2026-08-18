param(
    [string]$StateRoot = ''
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($StateRoot)) {
    $StateRoot = Join-Path $env:ProgramData 'Omniphony\production'
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the production Omniphony rollback from an elevated PowerShell session.'
}

$statePath = Join-Path $StateRoot 'installed-packages.json'
if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) {
    throw "No production install state exists at $statePath"
}
$state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
if ([string]$state.Schema -ne 'omniphony.windows.apo-installed.v4') {
    throw "Unsupported production install state schema: $($state.Schema)"
}

function Invoke-PnpUtil([string[]]$Arguments, [string]$Label) {
    Write-Host "PNPUTIL $Label"
    & "$env:WINDIR\System32\pnputil.exe" @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Label failed with pnputil exit code $LASTEXITCODE" }
}

function Get-XmlChildText($Node, [string]$Name) {
    if ($null -eq $Node) { return '' }
    $child = @($Node.ChildNodes | Where-Object { $_.LocalName -eq $Name } | Select-Object -First 1)
    if ($child.Count -eq 0) { return '' }
    return [string]$child[0].InnerText
}

function Get-OmniphonyPackages {
    $text = (& "$env:WINDIR\System32\pnputil.exe" /enum-drivers /devices /files /format xml 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) { throw "pnputil driver inventory failed with exit code $LASTEXITCODE" }
    $inventory = [xml]$text
    $targets = @('omniphonyapocomponent.inf', 'omniphonyapoextension.inf')
    return @(@($inventory.pnputil.driver) | ForEach-Object {
        $original = (Get-XmlChildText $_ 'originalName').ToLowerInvariant()
        if ($targets -contains $original) {
            [pscustomobject]@{
                OriginalName = Get-XmlChildText $_ 'originalName'
                PublishedName = Get-XmlChildText $_ 'publishedName'
                DriverVersion = Get-XmlChildText $_ 'driverVersion'
            }
        }
    } | Where-Object { $_ -and $_.PublishedName })
}

function Export-Packages($Packages, [string]$Root) {
    if (Test-Path -LiteralPath $Root) { Remove-Item -LiteralPath $Root -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $Root | Out-Null
    foreach ($package in @($Packages)) {
        Invoke-PnpUtil @('/export-driver', [string]$package.PublishedName, $Root) "export current $($package.PublishedName)"
    }
}

function Remove-Packages($Packages) {
    $ordered = @($Packages | Sort-Object @{
        Expression = { if ($_.OriginalName -ieq 'OmniphonyApoExtension.inf') { 0 } else { 1 } }
    })
    foreach ($package in $ordered) {
        Invoke-PnpUtil @('/delete-driver', [string]$package.PublishedName, '/uninstall', '/force') "remove $($package.PublishedName)"
    }
}

function Restore-Export([string]$Root) {
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) { throw "Rollback export is missing: $Root" }
    $infs = @(Get-ChildItem -LiteralPath $Root -Recurse -Filter '*.inf' -File | Sort-Object @{
        Expression = {
            if ($_.Name -ieq 'OmniphonyApoComponent.inf') { 0 }
            elseif ($_.Name -ieq 'OmniphonyApoExtension.inf') { 1 }
            else { 2 }
        }
    }, FullName)
    if ($infs.Count -eq 0) { throw "Rollback export contains no INF files: $Root" }
    foreach ($inf in $infs) {
        Invoke-PnpUtil @('/add-driver', $inf.FullName, '/install') "restore $($inf.Name)"
    }
}

function Restart-AudioGraph {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
    Start-Service -Name AudioSrv
    Start-Sleep -Milliseconds 800
}

$prior = @($state.PriorDriverPackages)
$rollbackRoot = [string]$state.RollbackExportRoot
if ($prior.Count -eq 0 -or [string]::IsNullOrWhiteSpace($rollbackRoot)) {
    Write-Host 'ROLLBACK_PREVIOUS_GENERATION 0'
    Write-Host 'This installation had no previous production Omniphony package generation. Use Uninstall-ProductionApoPackages.ps1 to return to the pre-Omniphony state.'
    exit 2
}
if (-not (Test-Path -LiteralPath $rollbackRoot -PathType Container)) {
    throw "Recorded rollback export no longer exists: $rollbackRoot"
}

$current = @(Get-OmniphonyPackages)
$safetyRoot = Join-Path $StateRoot ('rollback-current-safety\' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'))
Export-Packages $current $safetyRoot

try {
    Remove-Packages $current
    Restore-Export $rollbackRoot
    Invoke-PnpUtil @('/scan-devices') 'rollback rescan devices'
    Restart-AudioGraph

    $restored = @(Get-OmniphonyPackages)
    $expectedOriginal = @($prior | ForEach-Object { ([string]$_.OriginalName).ToLowerInvariant() } | Sort-Object -Unique)
    $actualOriginal = @($restored | ForEach-Object { ([string]$_.OriginalName).ToLowerInvariant() } | Sort-Object -Unique)
    foreach ($name in $expectedOriginal) {
        if ($actualOriginal -notcontains $name) {
            throw "Rollback did not restore expected package '$name'."
        }
    }

    $result = [ordered]@{
        Schema = 'omniphony.windows.apo-rollback.v1'
        RolledBackAtUtc = [DateTime]::UtcNow.ToString('o')
        FromInstallState = $statePath
        RestoredFrom = $rollbackRoot
        SafetyExportOfReplacedGeneration = $safetyRoot
        RestoredPackages = @($restored)
    }
    $resultPath = Join-Path $StateRoot 'rollback-last.json'
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $resultPath -Encoding UTF8

    Write-Host 'OMNIPHONY_PRODUCTION_ROLLBACK_OK 1'
    Write-Host "RESULT $resultPath"
    Write-Host "SAFETY_EXPORT $safetyRoot"
}
catch {
    $failure = $_
    Write-Warning "ROLLBACK_FAILED: $($failure.Exception.Message)"
    try {
        $partial = @(Get-OmniphonyPackages)
        Remove-Packages $partial
        Restore-Export $safetyRoot
        Invoke-PnpUtil @('/scan-devices') 'restore failed-rollback safety generation'
        Restart-AudioGraph
        Write-Warning 'The generation that existed immediately before rollback was restored from its safety export.'
    } catch {
        Write-Warning "Safety-generation restore also failed: $($_.Exception.Message)"
    }
    throw $failure
}
