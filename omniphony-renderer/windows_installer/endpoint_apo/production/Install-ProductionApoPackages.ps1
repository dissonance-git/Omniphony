param(
    [string]$PackageRoot = '',
    [string]$StateRoot = ''
)

$ErrorActionPreference = 'Stop'
$omniphonyClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'

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
$capturePath = Join-Path $PackageRoot 'target-capture.json'
foreach ($path in @($componentInf, $extensionInf, $manifestPath, $capturePath)) {
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

function Test-RequiredSignatures([string]$Root, $Manifest) {
    if (-not [bool]$Manifest.SignaturesVerified) {
        throw 'Package manifest does not record verified signatures. Production install refuses an unsigned or partially signed candidate.'
    }
    $paths = @(
        (Join-Path $Root 'component\OmniphonyAPO.dll'),
        (Join-Path $Root 'component\omniphony_realtime.dll'),
        (Join-Path $Root 'component\OmniphonyApo.cat'),
        (Join-Path $Root 'extension\OmniphonyApoExtension.cat')
    )
    foreach ($path in $paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required signed payload is missing: $path" }
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
            throw "Production payload signature is not locally Valid: $path status=$($signature.Status) $($signature.StatusMessage)"
        }
        Write-Host "SIGNATURE_READY`t$([IO.Path]::GetFileName($path))`t$($signature.SignerCertificate.Thumbprint)"
    }
}

function Get-StringArrayProperty($Item, [string]$Name) {
    if ($null -eq $Item) { return @() }
    $property = $Item.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) { return @() }
    return @($property.Value | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-EndpointEffectSnapshot($Capture) {
    $mmDeviceId = [string]$Capture.DefaultEndpoint.MmDeviceId
    if ($mmDeviceId -notmatch '(\{[0-9A-Fa-f-]{36}\})$') {
        throw "Captured MMDevice ID has no endpoint GUID tail: $mmDeviceId"
    }
    $endpointGuid = $Matches[1]
    $fxPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$endpointGuid\FxProperties"
    if (-not (Test-Path -LiteralPath $fxPath)) {
        throw "Captured endpoint FxProperties are not readable on this machine: $fxPath"
    }
    try { $item = Get-ItemProperty -LiteralPath $fxPath -ErrorAction Stop }
    catch { throw "Could not read captured endpoint FxProperties without modification: $($_.Exception.Message)" }

    $legacyName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
    $compositeName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},15'
    $disabledName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'
    $legacy = @(Get-StringArrayProperty $item $legacyName)
    $composite = @(Get-StringArrayProperty $item $compositeName)
    $disabled = 0
    $disabledProperty = $item.PSObject.Properties[$disabledName]
    if ($null -ne $disabledProperty -and $null -ne $disabledProperty.Value) {
        try { $disabled = [int]$disabledProperty.Value } catch { $disabled = -1 }
    }
    return [ordered]@{
        EndpointGuid = $endpointGuid
        RegistryPath = $fxPath
        LegacyEndpointEffects = $legacy
        CompositeEndpointEffects = $composite
        EnhancementsDisabled = $disabled
    }
}

function Test-LiveTarget($Capture) {
    if ([string]::IsNullOrWhiteSpace([string]$Capture.DefaultEndpoint.PnpInstanceId)) {
        throw 'Capture contains no default-endpoint PnP instance ID.'
    }
    $endpoint = Get-PnpDevice -InstanceId ([string]$Capture.DefaultEndpoint.PnpInstanceId) -PresentOnly -ErrorAction SilentlyContinue
    if (-not $endpoint) {
        throw "Captured default endpoint is no longer present: $($Capture.DefaultEndpoint.PnpInstanceId). Re-capture before installing."
    }

    $candidates = @($Capture.AssociationCandidates)
    if ($candidates.Count -ne 1) {
        throw "Bound capture must contain exactly one physical association candidate; got $($candidates.Count)."
    }
    $candidate = $candidates[0]
    $driver = Get-PnpDevice -InstanceId ([string]$candidate.InstanceId) -PresentOnly -ErrorAction SilentlyContinue
    if (-not $driver) {
        throw "Captured physical audio-driver candidate is no longer present: $($candidate.InstanceId). Re-capture before installing."
    }
    $liveHardwareIds = @()
    try { $liveHardwareIds = @((Get-PnpDeviceProperty -InstanceId ([string]$candidate.InstanceId) -KeyName 'DEVPKEY_Device_HardwareIds' -ErrorAction Stop).Data) }
    catch { throw "Could not re-read live hardware IDs for captured target: $($_.Exception.Message)" }
    $capturedIds = @($candidate.HardwareIds | ForEach-Object { ([string]$_).ToLowerInvariant() })
    $matches = @($liveHardwareIds | Where-Object { $capturedIds -contains ([string]$_).ToLowerInvariant() })
    if ($matches.Count -eq 0) {
        throw 'Live physical driver no longer exposes any hardware ID recorded in the bound capture. Re-capture before installing.'
    }

    $effects = Get-EndpointEffectSnapshot $Capture
    if ($effects.EnhancementsDisabled -eq 1) {
        throw 'Windows system effects are disabled for the captured endpoint. Production install refuses to claim success while the endpoint would bypass APO processing.'
    }
    $existing = @($effects.LegacyEndpointEffects + $effects.CompositeEndpointEffects | Sort-Object -Unique)
    $foreign = @($existing | Where-Object { -not ([string]$_).Equals($omniphonyClsid, [StringComparison]::OrdinalIgnoreCase) })
    if ($foreign.Count -gt 0) {
        throw "Captured endpoint already has non-Omniphony EFX registered: $($foreign -join ', '). Windows supports composite EFX, but Omniphony will not guess a safe ordering or overwrite a vendor effect."
    }
    return $effects
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
$expectedCaptureHash = ([string]$manifest.Capture.Sha256).ToLowerInvariant()
$actualCaptureHash = (Get-FileHash -LiteralPath $capturePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expectedCaptureHash -ne $actualCaptureHash) {
    throw 'Bound target capture does not match the capture hash recorded in package-manifest.json.'
}
Test-RequiredSignatures $PackageRoot $manifest
$capture = Get-Content -LiteralPath $capturePath -Raw | ConvertFrom-Json
$liveTargetBefore = Test-LiveTarget $capture
Write-Host "LIVE_TARGET_READY`t$($capture.DefaultEndpoint.FriendlyName)`t$($liveTargetBefore.EndpointGuid)"

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

    $liveTargetAfter = Get-EndpointEffectSnapshot $capture
    $postEffects = @($liveTargetAfter.LegacyEndpointEffects + $liveTargetAfter.CompositeEndpointEffects)
    $hasOmniphony = @($postEffects | Where-Object { ([string]$_).Equals($omniphonyClsid, [StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
    if (-not $hasOmniphony) {
        throw 'Driver packages installed, but the endpoint effects property store does not show the Omniphony EFX CLSID after AudioSrv restart.'
    }
    if ($liveTargetAfter.EnhancementsDisabled -eq 1) {
        throw 'Omniphony EFX is registered but Windows system effects are disabled on the endpoint.'
    }

    $state = [ordered]@{
        Schema = 'omniphony.windows.apo-installed.v2'
        InstalledAtUtc = [DateTime]::UtcNow.ToString('o')
        PackageRoot = $PackageRoot
        PackageManifestSha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
        TargetCaptureSha256 = $actualCaptureHash
        TargetEndpoint = [ordered]@{
            MmDeviceId = [string]$capture.DefaultEndpoint.MmDeviceId
            FriendlyName = [string]$capture.DefaultEndpoint.FriendlyName
            PnpInstanceId = [string]$capture.DefaultEndpoint.PnpInstanceId
            EndpointGuid = [string]$liveTargetAfter.EndpointGuid
        }
        PreInstallEffects = $liveTargetBefore
        PostInstallEffects = $liveTargetAfter
        ComponentDevices = @($devices | ForEach-Object {
            [ordered]@{ InstanceId = [string]$_.InstanceId; FriendlyName = [string]$_.FriendlyName; Class = [string]$_.Class }
        })
        DriverPackages = @($after)
        PriorDriverPackages = @($before)
    }
    $state | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $statePath -Encoding UTF8

    Write-Host ''
    Write-Host 'OMNIPHONY_PRODUCTION_INSTALL_OK 1'
    Write-Host "STATE $statePath"
    Write-Host "COMPONENT_DEVICES $($devices.Count)"
    Write-Host 'ENDPOINT_EFX_ASSOCIATION_OK 1'
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
