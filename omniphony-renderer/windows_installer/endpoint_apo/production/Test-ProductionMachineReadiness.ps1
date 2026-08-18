param(
    [string]$CaptureJson = '',
    [string]$PackageRoot = '',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$omniphonyClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'
$audioCategory = '{6994AD04-93EF-11D0-A3CC-00A0C9223196}'
$topologyCategory = '{DDA54A40-1E4C-11D1-A050-405705C10000}'

if (-not [string]::IsNullOrWhiteSpace($PackageRoot)) {
    $PackageRoot = [IO.Path]::GetFullPath($PackageRoot)
    if ([string]::IsNullOrWhiteSpace($CaptureJson)) {
        $CaptureJson = Join-Path $PackageRoot 'target-capture.json'
    }
}

function Get-RegistryValueIfPresent([string]$Path, [string]$Name) {
    try {
        $item = Get-ItemProperty -LiteralPath $Path -ErrorAction Stop
        if ($item.PSObject.Properties.Name -contains $Name) { return $item.$Name }
    } catch { }
    return $null
}

function Get-StringArrayProperty($Item, [string]$Name) {
    if ($null -eq $Item) { return @() }
    $property = $Item.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) { return @() }
    return @(
        $property.Value |
            ForEach-Object { [string]$_ } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    )
}

function Get-EndpointEffectSnapshot($Capture) {
    $mmDeviceId = [string]$Capture.DefaultEndpoint.MmDeviceId
    if ($mmDeviceId -notmatch '(\{[0-9A-Fa-f-]{36}\})$') {
        return [ordered]@{ Readable = $false; Error = "captured MMDevice ID has no endpoint GUID tail: $mmDeviceId" }
    }

    $endpointGuid = $Matches[1]
    $fxPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$endpointGuid\FxProperties"
    if (-not (Test-Path -LiteralPath $fxPath)) {
        return [ordered]@{
            Readable = $false
            EndpointGuid = $endpointGuid
            RegistryPath = $fxPath
            Error = 'endpoint FxProperties path is absent or unreadable'
        }
    }

    try { $item = Get-ItemProperty -LiteralPath $fxPath -ErrorAction Stop }
    catch {
        return [ordered]@{
            Readable = $false
            EndpointGuid = $endpointGuid
            RegistryPath = $fxPath
            Error = $_.Exception.Message
        }
    }

    $legacyName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
    $compositeName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},15'
    $disabledName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'
    $disabled = 0
    $disabledProperty = $item.PSObject.Properties[$disabledName]
    if ($null -ne $disabledProperty -and $null -ne $disabledProperty.Value) {
        try { $disabled = [int]$disabledProperty.Value } catch { $disabled = -1 }
    }

    return [ordered]@{
        Readable = $true
        EndpointGuid = $endpointGuid
        RegistryPath = $fxPath
        LegacyEndpointEffects = @(Get-StringArrayProperty $item $legacyName)
        CompositeEndpointEffects = @(Get-StringArrayProperty $item $compositeName)
        EnhancementsDisabled = $disabled
        Error = ''
    }
}

function Get-SafeTopologyReferences($Candidate) {
    $interfaces = @($Candidate.DriverInterfaces)
    $refs = @(
        $interfaces |
            ForEach-Object { [string]$_.ReferenceResolved } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            Sort-Object -Unique
    )

    $paired = New-Object System.Collections.Generic.List[string]
    foreach ($ref in $refs) {
        $categories = @(
            $interfaces |
                Where-Object { ([string]$_.ReferenceResolved) -ieq $ref } |
                ForEach-Object { [string]$_.CategoryResolved }
        )
        if (($categories -contains $audioCategory) -and ($categories -contains $topologyCategory)) {
            $paired.Add($ref)
        }
    }
    if ($paired.Count -gt 0) {
        return [ordered]@{
            References = @($paired | Sort-Object -Unique)
            Mode = 'paired-audio-topology'
        }
    }

    $legacy = @(
        $interfaces |
            Where-Object {
                ([string]$_.CategoryResolved).Equals($audioCategory, [StringComparison]::OrdinalIgnoreCase) -and
                ([string]$_.ReferenceResolved).Equals('Topology', [StringComparison]::OrdinalIgnoreCase)
            } |
            ForEach-Object { [string]$_.ReferenceResolved } |
            Sort-Object -Unique
    )
    return [ordered]@{
        References = $legacy
        Mode = if ($legacy.Count -gt 0) { 'legacy-kscategory-audio-topology' } else { '' }
    }
}

function Get-CaptureRecord([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return [ordered]@{ Supplied = $false; Exists = $false; Usable = $false; Error = ''; Data = $null }
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [ordered]@{ Supplied = $true; Exists = $false; Usable = $false; Error = 'capture file not found'; Data = $null }
    }

    try {
        $capture = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $candidates = @($capture.AssociationCandidates)
        $candidate = if ($candidates.Count -eq 1) { $candidates[0] } else { $null }
        $safeTopology = [ordered]@{ References = @(); Mode = '' }
        $warnings = @()
        $resolvedSection = ''
        $capturedSectionExt = ''
        $liveSectionExt = ''
        $endpointPresent = $false
        $driverPresent = $false
        $liveHardwareMatch = $false

        if ($null -ne $candidate) {
            $safeTopology = Get-SafeTopologyReferences $candidate
            $warnings = @(
                $candidate.InterfaceResolutionWarnings |
                    Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }
            )
            $resolvedSection = [string]$candidate.DriverInfResolvedSection
            $capturedSectionExt = [string]$candidate.DriverInfSectionExt

            $endpointPresent = $null -ne (
                Get-PnpDevice -InstanceId ([string]$capture.DefaultEndpoint.PnpInstanceId) -PresentOnly -ErrorAction SilentlyContinue
            )
            $driverPresent = $null -ne (
                Get-PnpDevice -InstanceId ([string]$candidate.InstanceId) -PresentOnly -ErrorAction SilentlyContinue
            )

            if ($driverPresent) {
                try {
                    $liveSectionExt = [string](
                        Get-PnpDeviceProperty -InstanceId ([string]$candidate.InstanceId) `
                            -KeyName 'DEVPKEY_Device_DriverInfSectionExt' -ErrorAction Stop
                    ).Data
                } catch { $liveSectionExt = '' }

                try {
                    $liveIds = @(
                        (Get-PnpDeviceProperty -InstanceId ([string]$candidate.InstanceId) `
                            -KeyName 'DEVPKEY_Device_HardwareIds' -ErrorAction Stop).Data
                    )
                    $capturedIds = @(
                        $candidate.HardwareIds |
                            ForEach-Object { ([string]$_).ToLowerInvariant() }
                    )
                    $liveHardwareMatch = @(
                        $liveIds |
                            Where-Object { $capturedIds -contains ([string]$_).ToLowerInvariant() }
                    ).Count -gt 0
                } catch { $liveHardwareMatch = $false }
            }
        }

        $capturedFx = $capture.CapturedEndpointEffects
        $capturedFxReadable = $null -ne $capturedFx -and [bool]$capturedFx.Readable
        $liveFx = Get-EndpointEffectSnapshot $capture
        $legacyDevEffect = $false
        $foreign = @()

        if ($liveFx.Readable) {
            $legacyDevEffect = @(
                $liveFx.LegacyEndpointEffects |
                    Where-Object { ([string]$_).Equals($omniphonyClsid, [StringComparison]::OrdinalIgnoreCase) }
            ).Count -gt 0

            $existing = @(
                $liveFx.LegacyEndpointEffects + $liveFx.CompositeEndpointEffects |
                    Sort-Object -Unique
            )
            $foreign = @(
                $existing |
                    Where-Object { -not ([string]$_).Equals($omniphonyClsid, [StringComparison]::OrdinalIgnoreCase) }
            )
        }

        $usable = ([string]$capture.Schema -eq 'omniphony.windows.apo-target.v3') -and
            ($candidates.Count -eq 1) -and
            (@($safeTopology.References).Count -eq 1) -and
            (@($candidate.HardwareIds).Count -gt 0) -and
            (-not [string]::IsNullOrWhiteSpace($resolvedSection)) -and
            ($warnings.Count -eq 0) -and
            $capturedFxReadable -and
            $endpointPresent -and
            $driverPresent -and
            $liveHardwareMatch -and
            $liveFx.Readable -and
            ($liveFx.EnhancementsDisabled -ne 1) -and
            (-not $legacyDevEffect) -and
            ($foreign.Count -eq 0) -and
            $capturedSectionExt.Equals($liveSectionExt, [StringComparison]::OrdinalIgnoreCase)

        return [ordered]@{
            Supplied = $true
            Exists = $true
            Path = (Resolve-Path -LiteralPath $Path).Path
            Schema = [string]$capture.Schema
            DefaultEndpoint = [string]$capture.DefaultEndpoint.FriendlyName
            MmDeviceId = [string]$capture.DefaultEndpoint.MmDeviceId
            CandidateCount = $candidates.Count
            SafeTopologyReferences = @($safeTopology.References)
            TopologyEvidenceMode = [string]$safeTopology.Mode
            ResolvedDriverSection = $resolvedSection
            CapturedDriverSectionExt = $capturedSectionExt
            LiveDriverSectionExt = $liveSectionExt
            InterfaceResolutionWarnings = $warnings
            EndpointPresent = [bool]$endpointPresent
            DriverPresent = [bool]$driverPresent
            LiveHardwareMatch = [bool]$liveHardwareMatch
            CapturedEffectsReadable = [bool]$capturedFxReadable
            LiveEndpointEffects = $liveFx
            LegacyDevelopmentEffectPresent = [bool]$legacyDevEffect
            ForeignEndpointEffects = $foreign
            Usable = [bool]$usable
            Error = ''
            Data = $capture
        }
    } catch {
        return [ordered]@{
            Supplied = $true
            Exists = $true
            Usable = $false
            Error = $_.Exception.Message
            Data = $null
        }
    }
}

function Get-SignatureRecord([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [ordered]@{ Path = $Path; Exists = $false; Status = 'Missing'; Thumbprint = ''; Subject = '' }
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    return [ordered]@{
        Path = (Resolve-Path -LiteralPath $Path).Path
        Exists = $true
        Status = [string]$signature.Status
        Thumbprint = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { '' }
        Subject = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { '' }
    }
}

function Get-PackageRecord([string]$Root) {
    if ([string]::IsNullOrWhiteSpace($Root)) {
        return [ordered]@{ Supplied = $false; Exists = $false; ManifestValid = $false; SignaturesValid = $false; Error = '' }
    }
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        return [ordered]@{ Supplied = $true; Exists = $false; ManifestValid = $false; SignaturesValid = $false; Error = 'package root not found' }
    }

    try {
        $rootPath = (Resolve-Path -LiteralPath $Root).Path
        $manifestPath = Join-Path $rootPath 'package-manifest.json'
        if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
            throw 'package-manifest.json is missing'
        }

        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ([string]$manifest.Schema -ne 'omniphony.windows.apo-package-build.v2') {
            throw "unsupported package manifest schema: $($manifest.Schema)"
        }

        $manifestValid = $true
        foreach ($entry in @($manifest.Files)) {
            $path = Join-Path $rootPath ([string]$entry.Path)
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                $manifestValid = $false
                continue
            }
            $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($actual -ne ([string]$entry.Sha256).ToLowerInvariant()) { $manifestValid = $false }
        }

        $probePath = Join-Path $rootPath 'diagnostics\OmniphonyProductionProbe.exe'
        if ([string]$manifest.ProductionProbePath -ne 'diagnostics\OmniphonyProductionProbe.exe') {
            $manifestValid = $false
        }

        $signaturePaths = @(
            (Join-Path $rootPath 'component\OmniphonyAPO.dll'),
            (Join-Path $rootPath 'component\omniphony_realtime.dll'),
            (Join-Path $rootPath 'component\OmniphonyApo.cat'),
            (Join-Path $rootPath 'extension\OmniphonyApoExtension.cat'),
            $probePath
        )
        $signatures = @($signaturePaths | ForEach-Object { Get-SignatureRecord $_ })
        $signaturesValid = ($signatures.Count -eq 5) -and
            (@($signatures | Where-Object { $_.Status -ne 'Valid' }).Count -eq 0)

        return [ordered]@{
            Supplied = $true
            Exists = $true
            Path = $rootPath
            ManifestSchema = [string]$manifest.Schema
            ManifestValid = [bool]$manifestValid
            ManifestSaysSignaturesVerified = [bool]$manifest.SignaturesVerified
            SignaturesValid = [bool]$signaturesValid
            ProbePath = $probePath
            Files = $signatures
            Error = ''
        }
    } catch {
        return [ordered]@{
            Supplied = $true
            Exists = $true
            ManifestValid = $false
            SignaturesValid = $false
            Error = $_.Exception.Message
        }
    }
}

function Invoke-ReadOnlyWasapiProbe([string]$ProbePath, [string]$MmDeviceId) {
    if ([string]::IsNullOrWhiteSpace($ProbePath) -or -not (Test-Path -LiteralPath $ProbePath -PathType Leaf)) {
        return [ordered]@{ Ran = $false; Passed = $false; ExitCode = $null; Output = @(); Error = 'production probe is missing' }
    }
    if ([string]::IsNullOrWhiteSpace($MmDeviceId)) {
        return [ordered]@{ Ran = $false; Passed = $false; ExitCode = $null; Output = @(); Error = 'capture has no MMDevice ID' }
    }

    try {
        $lines = @(& $ProbePath $MmDeviceId 2>&1 | ForEach-Object { "$_" })
        $code = $LASTEXITCODE
        $success = $lines |
            Where-Object { $_ -eq "OMNIPHONY_PRODUCTION_WASAPI_PROBE_OK`t1" } |
            Select-Object -First 1
        return [ordered]@{
            Ran = $true
            Passed = ($code -eq 0 -and $null -ne $success)
            ExitCode = $code
            Output = @($lines)
            Error = if ($code -eq 0 -and $null -ne $success) { '' } else { $lines -join ' | ' }
        }
    } catch {
        return [ordered]@{ Ran = $true; Passed = $false; ExitCode = $null; Output = @(); Error = $_.Exception.Message }
    }
}

$os = Get-CimInstance Win32_OperatingSystem
$build = [int]$os.BuildNumber
$windows11Eligible = $build -ge 22000

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

$audioReg = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$disableProtectedAudioDG = Get-RegistryValueIfPresent $audioReg 'DisableProtectedAudioDG'
$protectedAudioDgBypassActive = ($disableProtectedAudioDG -eq 1)

$capture = Get-CaptureRecord $CaptureJson
$package = Get-PackageRecord $PackageRoot
$baselineProbe = if (
    $PackageRoot -and
    $capture.Usable -and
    $package.ManifestValid -and
    $package.SignaturesValid -and
    $package.ManifestSaysSignaturesVerified
) {
    Invoke-ReadOnlyWasapiProbe $package.ProbePath $capture.MmDeviceId
} else {
    [ordered]@{
        Ran = $false
        Passed = $false
        ExitCode = $null
        Output = @()
        Error = 'prerequisite gates did not pass'
    }
}

$blockers = New-Object System.Collections.Generic.List[string]
if (-not $windows11Eligible) {
    $blockers.Add('Windows build is below 22000; the production APO package targets Windows 11 21H2 or later.')
}
if (-not $isAdmin) {
    $blockers.Add('Run production readiness from an elevated PowerShell session.')
}
if ($protectedAudioDgBypassActive) {
    $blockers.Add('DisableProtectedAudioDG=1 is active. Remove the development APO before production testing.')
}
if ($CaptureJson -and -not $capture.Usable) {
    $blockers.Add('The target capture is not live, unambiguous v3 evidence with one safe topology association, matching hardware/driver state, enabled effects, and no legacy or foreign endpoint EFX.')
}
if ($PackageRoot -and -not $package.ManifestValid) {
    $blockers.Add('The supplied v2 package manifest or payload hashes are invalid.')
}
if ($PackageRoot -and -not $package.SignaturesValid) {
    $blockers.Add('The production candidate does not have five locally Valid Authenticode signatures, including the production WASAPI probe.')
}
if ($PackageRoot -and -not $package.ManifestSaysSignaturesVerified) {
    $blockers.Add('The package manifest does not record completed signature verification.')
}
if (
    $PackageRoot -and
    $capture.Usable -and
    $package.ManifestValid -and
    $package.SignaturesValid -and
    $package.ManifestSaysSignaturesVerified -and
    -not $baselineProbe.Passed
) {
    $blockers.Add('The exact captured endpoint fails the read-only pre-install WASAPI/GetMixFormat/shared-render probe.')
}

$report = [ordered]@{
    Schema = 'omniphony.windows.apo-readiness.v3'
    CheckedAtUtc = [DateTime]::UtcNow.ToString('o')
    Machine = [ordered]@{
        Caption = [string]$os.Caption
        Version = [string]$os.Version
        BuildNumber = $build
        Windows11ApoClassEligible = [bool]$windows11Eligible
        Elevated = [bool]$isAdmin
    }
    AudioDG = [ordered]@{
        DisableProtectedAudioDGValue = $disableProtectedAudioDG
        DevelopmentBypassActive = [bool]$protectedAudioDgBypassActive
    }
    Capture = $capture
    Package = $package
    BaselineWasapiProbe = $baselineProbe
    Blockers = @($blockers)
    RepositorySideReadyForPhysicalTest = (
        $blockers.Count -eq 0 -and
        $capture.Usable -and
        $package.ManifestValid -and
        $package.SignaturesValid -and
        $package.ManifestSaysSignaturesVerified -and
        $baselineProbe.Passed
    )
    Note = 'Zero blockers means the machine is ready to attempt the protected DriverStore transaction. Protected AudioDG load is proven only by the post-install probe.'
}

$json = $report | ConvertTo-Json -Depth 14
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $fullOutput = [IO.Path]::GetFullPath($OutputPath)
    $parent = Split-Path -Parent $fullOutput
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json | Set-Content -LiteralPath $fullOutput -Encoding UTF8
    Write-Host "OMNIPHONY_PRODUCTION_READINESS_REPORT`t$fullOutput"
}
$json

if ($blockers.Count -eq 0) {
    Write-Host 'OMNIPHONY_PRODUCTION_PREFLIGHT_BLOCKERS 0'
} else {
    Write-Warning "OMNIPHONY_PRODUCTION_PREFLIGHT_BLOCKERS $($blockers.Count)"
}
