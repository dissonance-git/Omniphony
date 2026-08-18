param(
    [string]$CaptureJson = '',
    [string]$PackageRoot = '',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'

function Get-RegistryValueIfPresent([string]$Path, [string]$Name) {
    try {
        $item = Get-ItemProperty -LiteralPath $Path -ErrorAction Stop
        if ($item.PSObject.Properties.Name -contains $Name) {
            return $item.$Name
        }
    } catch { }
    return $null
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

function Get-PnpDriverInventory {
    $pnputil = Join-Path $env:WINDIR 'System32\pnputil.exe'
    $text = (& $pnputil /enum-drivers /devices /files /format xml 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) {
        return [ordered]@{ Available = $false; Error = "pnputil exit $LASTEXITCODE"; Omniphony = @() }
    }
    try {
        $xml = [xml]$text
        $matches = @($xml.pnputil.driver | Where-Object {
            $raw = $_.OuterXml.ToLowerInvariant()
            $raw.Contains('omniphonyapocomponent.inf') -or
            $raw.Contains('omniphonyapoextension.inf') -or
            $raw.Contains('ven_omni&amp;cid_current') -or
            $raw.Contains('ven_omni&cid_current')
        } | ForEach-Object {
            [ordered]@{ Xml = $_.OuterXml }
        })
        return [ordered]@{ Available = $true; Error = ''; Omniphony = $matches }
    } catch {
        return [ordered]@{ Available = $false; Error = "invalid pnputil XML: $($_.Exception.Message)"; Omniphony = @() }
    }
}

function Get-CaptureRecord([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return [ordered]@{ Supplied = $false; Exists = $false; Schema = ''; CandidateCount = 0; PairedTopologyReferences = @(); Usable = $false; Error = '' }
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [ordered]@{ Supplied = $true; Exists = $false; Schema = ''; CandidateCount = 0; PairedTopologyReferences = @(); Usable = $false; Error = 'capture file not found' }
    }
    try {
        $capture = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $candidates = @($capture.AssociationCandidates)
        $audio = '{6994AD04-93EF-11D0-A3CC-00A0C9223196}'
        $topology = '{DDA54A40-1E4C-11D1-A050-405705C10000}'
        $paired = New-Object System.Collections.Generic.List[string]
        if ($candidates.Count -eq 1) {
            $interfaces = @($candidates[0].DriverInterfaces)
            $refs = @($interfaces | ForEach-Object { [string]$_.ReferenceResolved } | Where-Object { $_ } | Sort-Object -Unique)
            foreach ($ref in $refs) {
                $categories = @($interfaces | Where-Object { ([string]$_.ReferenceResolved) -ieq $ref } | ForEach-Object { [string]$_.CategoryResolved })
                if (($categories -contains $audio) -and ($categories -contains $topology)) {
                    $paired.Add($ref)
                }
            }
        }
        $usable = ([string]$capture.Schema -eq 'omniphony.windows.apo-target.v2') -and
            ($candidates.Count -eq 1) -and
            ($paired.Count -eq 1) -and
            (@($candidates[0].HardwareIds).Count -gt 0)
        return [ordered]@{
            Supplied = $true
            Exists = $true
            Path = (Resolve-Path -LiteralPath $Path).Path
            Schema = [string]$capture.Schema
            DefaultEndpoint = [string]$capture.DefaultEndpoint.FriendlyName
            CandidateCount = $candidates.Count
            PairedTopologyReferences = @($paired)
            Usable = [bool]$usable
            Error = ''
        }
    } catch {
        return [ordered]@{ Supplied = $true; Exists = $true; Schema = ''; CandidateCount = 0; PairedTopologyReferences = @(); Usable = $false; Error = $_.Exception.Message }
    }
}

function Get-PackageRecord([string]$Root) {
    if ([string]::IsNullOrWhiteSpace($Root)) {
        return [ordered]@{ Supplied = $false; Exists = $false; ManifestValid = $false; SignaturesValid = $false; Files = @(); Error = '' }
    }
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        return [ordered]@{ Supplied = $true; Exists = $false; ManifestValid = $false; SignaturesValid = $false; Files = @(); Error = 'package root not found' }
    }
    try {
        $rootPath = (Resolve-Path -LiteralPath $Root).Path
        $manifestPath = Join-Path $rootPath 'package-manifest.json'
        if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
            throw 'package-manifest.json is missing'
        }
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ($manifest.Schema -ne 'omniphony.windows.apo-package-build.v1') {
            throw "unsupported package manifest schema: $($manifest.Schema)"
        }
        $manifestValid = $true
        foreach ($entry in @($manifest.Files)) {
            $path = Join-Path $rootPath ([string]$entry.Path)
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                $manifestValid = $false
                continue
            }
            $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($hash -ne ([string]$entry.Sha256).ToLowerInvariant()) { $manifestValid = $false }
        }

        $signaturePaths = @(
            (Join-Path $rootPath 'component\OmniphonyAPO.dll'),
            (Join-Path $rootPath 'component\omniphony_realtime.dll'),
            (Join-Path $rootPath 'component\OmniphonyApo.cat'),
            (Join-Path $rootPath 'extension\OmniphonyApoExtension.cat')
        )
        $signatures = @($signaturePaths | ForEach-Object { Get-SignatureRecord $_ })
        $signaturesValid = ($signatures.Count -eq 4) -and (@($signatures | Where-Object { $_.Status -ne 'Valid' }).Count -eq 0)
        return [ordered]@{
            Supplied = $true
            Exists = $true
            Path = $rootPath
            ManifestValid = [bool]$manifestValid
            ManifestSaysSignaturesVerified = [bool]$manifest.SignaturesVerified
            SignaturesValid = [bool]$signaturesValid
            Files = $signatures
            Error = ''
        }
    } catch {
        return [ordered]@{ Supplied = $true; Exists = $true; ManifestValid = $false; SignaturesValid = $false; Files = @(); Error = $_.Exception.Message }
    }
}

$os = Get-CimInstance Win32_OperatingSystem
$build = [int]$os.BuildNumber
$windows11Eligible = $build -ge 22000

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

$audioReg = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
$disableProtectedAudioDG = Get-RegistryValueIfPresent $audioReg 'DisableProtectedAudioDG'
$protectedAudioDgBypassActive = ($disableProtectedAudioDG -eq 1)

$secureBoot = 'Unknown'
try {
    $secureBoot = if (Confirm-SecureBootUEFI -ErrorAction Stop) { 'Enabled' } else { 'Disabled' }
} catch {
    $secureBoot = 'UnavailableOrUnsupported'
}

$deviceGuard = $null
try {
    $dg = Get-CimInstance -Namespace 'root\Microsoft\Windows\DeviceGuard' -ClassName Win32_DeviceGuard -ErrorAction Stop
    $deviceGuard = [ordered]@{
        VirtualizationBasedSecurityStatus = [int]$dg.VirtualizationBasedSecurityStatus
        SecurityServicesConfigured = @($dg.SecurityServicesConfigured)
        SecurityServicesRunning = @($dg.SecurityServicesRunning)
    }
} catch {
    $deviceGuard = [ordered]@{ Error = $_.Exception.Message }
}

$capture = Get-CaptureRecord $CaptureJson
$package = Get-PackageRecord $PackageRoot
$driverInventory = Get-PnpDriverInventory

$blockers = New-Object System.Collections.Generic.List[string]
if (-not $windows11Eligible) { $blockers.Add('Windows build is below 22000; the production APO package targets Windows 11 21H2 or later.') }
if ($protectedAudioDgBypassActive) { $blockers.Add('DisableProtectedAudioDG=1 is active. Remove the development bypass before production testing.') }
if ($CaptureJson -and -not $capture.Usable) { $blockers.Add('The supplied target capture is not unambiguous v2 evidence with one hardware candidate and one paired topology reference.') }
if ($PackageRoot -and -not $package.ManifestValid) { $blockers.Add('The supplied package manifest or payload hashes are invalid.') }
if ($PackageRoot -and -not $package.SignaturesValid) { $blockers.Add('The supplied production candidate does not have four locally Valid Authenticode signatures.') }

$report = [ordered]@{
    Schema = 'omniphony.windows.apo-readiness.v1'
    CheckedAtUtc = [DateTime]::UtcNow.ToString('o')
    Machine = [ordered]@{
        Caption = [string]$os.Caption
        Version = [string]$os.Version
        BuildNumber = $build
        Windows11ApoClassEligible = [bool]$windows11Eligible
        Elevated = [bool]$isAdmin
        SecureBoot = $secureBoot
        DeviceGuard = $deviceGuard
    }
    AudioDG = [ordered]@{
        DisableProtectedAudioDGValue = $disableProtectedAudioDG
        DevelopmentBypassActive = [bool]$protectedAudioDgBypassActive
    }
    DriverStore = $driverInventory
    Capture = $capture
    Package = $package
    Blockers = @($blockers)
    RepositorySideReadyForPhysicalTest = ($blockers.Count -eq 0 -and $capture.Usable -and $package.ManifestValid -and $package.SignaturesValid)
    Note = 'A clean readiness report is necessary evidence only. It does not prove that Windows protected AudioDG will load the APO or that physical playback succeeds.'
}

$json = $report | ConvertTo-Json -Depth 12
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
    $parent = Split-Path -Parent ([IO.Path]::GetFullPath($OutputPath))
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    $json | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    Write-Host "OMNIPHONY_PRODUCTION_READINESS_REPORT $OutputPath"
}
$json

if ($blockers.Count -eq 0) {
    Write-Host 'OMNIPHONY_PRODUCTION_PREFLIGHT_BLOCKERS 0'
} else {
    Write-Warning "OMNIPHONY_PRODUCTION_PREFLIGHT_BLOCKERS $($blockers.Count)"
}
