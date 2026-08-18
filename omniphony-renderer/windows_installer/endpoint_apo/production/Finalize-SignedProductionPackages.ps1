param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,
    [switch]$RequireMicrosoftCatalogSigner
)

$ErrorActionPreference = 'Stop'
$PackageRoot = (Resolve-Path -LiteralPath $PackageRoot -ErrorAction Stop).Path

function Get-FileRecord([string]$Path) {
    $item = Get-Item -LiteralPath $Path -ErrorAction Stop
    return [ordered]@{
        Name = $item.Name
        Length = [long]$item.Length
        Sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Get-SignatureRecord([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required signed production payload is missing: $Path"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Production payload signature is not locally Valid: $Path status=$($signature.Status) $($signature.StatusMessage)"
    }
    return [ordered]@{
        Path = $Path
        Status = [string]$signature.Status
        Thumbprint = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Thumbprint } else { '' }
        Subject = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Subject } else { '' }
        Issuer = if ($signature.SignerCertificate) { [string]$signature.SignerCertificate.Issuer } else { '' }
    }
}

$componentInf = Join-Path $PackageRoot 'component\OmniphonyApoComponent.inf'
$extensionInf = Join-Path $PackageRoot 'extension\OmniphonyApoExtension.inf'
$capturePath = Join-Path $PackageRoot 'target-capture.json'
$probePath = Join-Path $PackageRoot 'diagnostics\OmniphonyProductionProbe.exe'
$componentApo = Join-Path $PackageRoot 'component\OmniphonyAPO.dll'
$componentRealtime = Join-Path $PackageRoot 'component\omniphony_realtime.dll'
$componentCat = Join-Path $PackageRoot 'component\OmniphonyApo.cat'
$extensionCat = Join-Path $PackageRoot 'extension\OmniphonyApoExtension.cat'

foreach ($required in @(
    $componentInf,
    $extensionInf,
    $capturePath,
    $probePath,
    $componentApo,
    $componentRealtime,
    $componentCat,
    $extensionCat
)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Signed production package is incomplete: $required"
    }
}

$capture = Get-Content -LiteralPath $capturePath -Raw | ConvertFrom-Json
if ([string]$capture.Schema -ne 'omniphony.windows.apo-target.v3') {
    throw "Signed package contains unsupported target evidence schema: $($capture.Schema)"
}

$signaturePaths = @($componentApo, $componentRealtime, $componentCat, $extensionCat, $probePath)
$signatures = @($signaturePaths | ForEach-Object { Get-SignatureRecord $_ })

if ($RequireMicrosoftCatalogSigner) {
    foreach ($catalog in @($componentCat, $extensionCat)) {
        $record = Get-SignatureRecord $catalog
        $identity = ($record.Subject + ' ' + $record.Issuer)
        if ($identity -notmatch '(?i)Microsoft') {
            throw "Catalog does not appear to carry a Microsoft production/test signing identity: $catalog subject=$($record.Subject) issuer=$($record.Issuer)"
        }
    }
}

$files = New-Object System.Collections.Generic.List[object]
Get-ChildItem -LiteralPath $PackageRoot -Recurse -File |
    Where-Object { $_.Name -ne 'package-manifest.json' } |
    Sort-Object FullName |
    ForEach-Object {
        $rootPrefix = $PackageRoot.TrimEnd('\') + '\'
        if (-not $_.FullName.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Package file escaped package root: $($_.FullName)"
        }
        $relative = $_.FullName.Substring($rootPrefix.Length)
        $record = Get-FileRecord $_.FullName
        $files.Add([ordered]@{
            Path = $relative
            Length = $record.Length
            Sha256 = $record.Sha256
        })
    }

$manifest = [ordered]@{
    Schema = 'omniphony.windows.apo-package-build.v2'
    BuiltAtUtc = [DateTime]::UtcNow.ToString('o')
    FinalizedAfterExternalSigning = $true
    Capture = Get-FileRecord $capturePath
    CapturePath = 'target-capture.json'
    ProductionProbePath = 'diagnostics\OmniphonyProductionProbe.exe'
    CatalogsGenerated = $true
    CertificateThumbprint = $null
    SignaturesVerified = $true
    SignatureRecords = $signatures
    Files = @($files)
}

$manifestPath = Join-Path $PackageRoot 'package-manifest.json'
$manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

Write-Host 'OMNIPHONY_SIGNED_PACKAGE_FINALIZED 1'
Write-Host "PACKAGE_ROOT`t$PackageRoot"
Write-Host "MANIFEST`t$manifestPath"
if ($RequireMicrosoftCatalogSigner) {
    Write-Host 'MICROSOFT_CATALOG_SIGNATURE_OBSERVED 1'
}
