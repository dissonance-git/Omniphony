param(
    [string]$PackageRoot = '',
    [string]$StateRoot = ''
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($PackageRoot)) {
    $PackageRoot = Join-Path (Get-Location) 'omniphony-production-packages'
}
if ([string]::IsNullOrWhiteSpace($StateRoot)) {
    $StateRoot = Join-Path $env:ProgramData 'Omniphony\production'
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the production Omniphony installer from an elevated PowerShell session.'
}

$PackageRoot = (Resolve-Path -LiteralPath $PackageRoot).Path
$componentInf = Join-Path $PackageRoot 'component\OmniphonyApoComponent.inf'
$extensionInf = Join-Path $PackageRoot 'extension\OmniphonyApoExtension.inf'
$manifestPath = Join-Path $PackageRoot 'package-manifest.json'
foreach ($path in @($componentInf, $extensionInf, $manifestPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing production package file: $path" }
}

$audioProtectionPath = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$audioProtection = Get-ItemProperty -LiteralPath $audioProtectionPath -ErrorAction Stop
$disableProtected = $null
if ($audioProtection.PSObject.Properties.Name -contains 'DisableProtectedAudioDG') {
    $disableProtected = [int]$audioProtection.DisableProtectedAudioDG
}
if ($disableProtected -eq 1) {
    throw 'DisableProtectedAudioDG=1 is still active. Remove/uninstall the development APO path first. Production installation refuses to hide signing or package problems behind the test bypass.'
}

function Invoke-PnpUtil([string[]]$Arguments, [string]$Label) {
    Write-Host "PNPUTIL $Label"
    & "$env:WINDIR\System32\pnputil.exe" @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Label failed with pnputil exit code $LASTEXITCODE" }
}

function Get-DriverInventory {
    $text = (& "$env:WINDIR\System32\pnputil.exe" /enum-drivers /devices /files /format xml 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) { throw "pnputil driver inventory failed with exit code $LASTEXITCODE" }
    try { return [xml]$text } catch { throw "pnputil returned invalid XML driver inventory: $($_.Exception.Message)" }
}

function Get-XmlChildText($Node, [string]$Name) {
    if ($null -eq $Node) { return '' }
    $child = @($Node.ChildNodes | Where-Object { $_.LocalName -eq $Name } | Select-Object -First 1)
    if ($child.Count -eq 0) { return '' }
    return [string]$child[0].InnerText
}

function Get-OmniphonyPackages($Inventory) {
    $targets = @('omniphonyapocomponent.inf', 'omniphonyapoextension.inf')
    $drivers = @($Inventory.pnputil.driver)
    return @($drivers | ForEach-Object {
        $original = (Get-XmlChildText $_ 'originalName').ToLowerInvariant()
        if ($targets -contains $original) {
            [pscustomobject]@{
                OriginalName = Get-XmlChildText $_ 'originalName'
                PublishedName = Get-XmlChildText $_ 'publishedName'
                ProviderName = Get-XmlChildText $_ 'providerName'
                ClassName = Get-XmlChildText $_ 'className'
                DriverVersion = Get-XmlChildText $_ 'driverVersion'
            }
        }
    } | Where-Object { $_ })
}

function Test-PackageManifest([string]$Root, [string]$Path) {
    $manifest = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if ($manifest.Schema -ne 'omniphony.windows.apo-package-build.v1') {
        throw "Unsupported package manifest schema: $($manifest.Schema)"
    }
    foreach ($record in @($manifest.Files)) {
        $file = Join-Path $Root ([string]$record.Path)
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Manifest payload is missing: $file" }
        $actual = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne ([string]$record.Sha256).ToLowerInvariant()) {
            throw "Manifest hash mismatch: $file"
        }
    }
    return $manifest
}

function Restart-AudioGraph {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
    Start-Service -Name AudioSrv
    Start-Sleep -Milliseconds 800
}

function Get-OmniphonyComponentDevices {
    return @(Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object {
        ([string]$_.InstanceId).StartsWith('SWC\VEN_OMNI&CID_CURRENT', [StringComparison]::OrdinalIgnoreCase)
    })
}

function Remove-NewPackages($BeforePublishedNames) {
    try {
        $inventory = Get-DriverInventory
        $packages = @(Get-OmniphonyPackages $inventory)
        $newPackages = @($packages | Where-Object {
            $_.PublishedName -and -not $BeforePublishedNames.ContainsKey($_.PublishedName.ToLowerInvariant())
        } | Sort-Object @{ Expression = { if ($_.OriginalName -ieq 'OmniphonyApoExtension.inf') { 0 } else { 1 } } })
        foreach ($package in $newPackages) {
            try {
                Invoke-PnpUtil @('/delete-driver', $package.PublishedName, '/uninstall', '/force') "rollback $($package.PublishedName)"
            } catch {
                Write-Warning "Rollback could not remove $($package.PublishedName): $($_.Exception.Message)"
            }
        }
    } catch {
        Write-Warning "Could not inventory rollback packages: $($_.Exception.Message)"
    }
}

$manifest = Test-PackageManifest $PackageRoot $manifestPath
New-Item -ItemType Directory -Force -Path $StateRoot | Out-Null
$statePath = Join-Path $StateRoot 'installed-packages.json'
$logPath = Join-Path $StateRoot 'install-last.log'
$transcript = $false
try {
    Start-Transcript -Path $logPath -Force | Out-Null
    $transcript = $true
} catch {
    Write-Warning "Could not start production install transcript: $($_.Exception.Message)"
}

$before = @(Get-OmniphonyPackages (Get-DriverInventory))
$beforeNames = @{}
foreach ($package in $before) {
    if ($package.PublishedName) { $beforeNames[$package.PublishedName.ToLowerInvariant()] = $true }
}

try {
    Invoke-PnpUtil @('/add-driver', $componentInf, '/install') 'stage/install APO component package'
    Invoke-PnpUtil @('/add-driver', $extensionInf, '/install') 'stage/install audio-driver extension package'
    Invoke-PnpUtil @('/scan-devices') 'rescan devices'
    Restart-AudioGraph

    $devices = @(Get-OmniphonyComponentDevices)
    if ($devices.Count -eq 0) {
        throw 'Windows did not create the SWC\VEN_OMNI&CID_CURRENT APO component after package installation.'
    }

    $after = @(Get-OmniphonyPackages (Get-DriverInventory))
    $componentPackage = @($after | Where-Object { $_.OriginalName -ieq 'OmniphonyApoComponent.inf' })
    $extensionPackage = @($after | Where-Object { $_.OriginalName -ieq 'OmniphonyApoExtension.inf' })
    if ($componentPackage.Count -eq 0 -or $extensionPackage.Count -eq 0) {
        throw 'PnP driver inventory does not contain both Omniphony production packages after installation.'
    }

    $state = [ordered]@{
        Schema = 'omniphony.windows.apo-installed.v1'
        InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
        PackageRoot = $PackageRoot
        PackageManifestSha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
        ComponentDevices = @($devices | ForEach-Object {
            [ordered]@{ InstanceId = [string]$_.InstanceId; FriendlyName = [string]$_.FriendlyName; Class = [string]$_.Class }
        })
        DriverPackages = @($after)
        PriorDriverPackages = @($before)
    }
    $state | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statePath -Encoding UTF8

    Write-Host ''
    Write-Host 'OMNIPHONY_PRODUCTION_INSTALL_OK 1'
    Write-Host "STATE $statePath"
    Write-Host "COMPONENT_DEVICES $($devices.Count)"
    Write-Host 'Protected AudioDG bypass remains disabled.'
}
catch {
    $failure = $_
    Write-Warning "OMNIPHONY_PRODUCTION_INSTALL_FAILED: $($failure.Exception.Message)"
    Remove-NewPackages $beforeNames
    try { Invoke-PnpUtil @('/scan-devices') 'rollback rescan devices' } catch { Write-Warning $_.Exception.Message }
    try { Restart-AudioGraph } catch { Write-Warning "Audio graph rollback restart failed: $($_.Exception.Message)" }
    throw $failure
}
finally {
    if ($transcript) { try { Stop-Transcript | Out-Null } catch { } }
}
