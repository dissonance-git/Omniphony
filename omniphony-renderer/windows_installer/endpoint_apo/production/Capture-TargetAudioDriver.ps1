param(
    [string]$EndpointCtl = '',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($EndpointCtl)) {
    $candidates = @(
        (Join-Path $env:ProgramFiles 'Omniphony\support\OmniphonyEndpointCtl.exe'),
        (Join-Path $PSScriptRoot '..\OmniphonyEndpointCtl.exe')
    )
    $EndpointCtl = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if ([string]::IsNullOrWhiteSpace($EndpointCtl) -or -not (Test-Path -LiteralPath $EndpointCtl)) {
    throw 'OmniphonyEndpointCtl.exe was not found. Pass -EndpointCtl with the built or installed helper path.'
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path (Get-Location) 'omniphony-audio-target.json'
}

function Get-PnpPropertyData([string]$InstanceId, [string]$KeyName) {
    try {
        return (Get-PnpDeviceProperty -InstanceId $InstanceId -KeyName $KeyName -ErrorAction Stop).Data
    } catch {
        return $null
    }
}

function Resolve-InstalledInfPath([string]$DriverInfPath) {
    if ([string]::IsNullOrWhiteSpace($DriverInfPath)) { return '' }
    if ([IO.Path]::IsPathRooted($DriverInfPath) -and (Test-Path -LiteralPath $DriverInfPath)) {
        return (Resolve-Path -LiteralPath $DriverInfPath).Path
    }
    $windowsInf = Join-Path $env:WINDIR ('INF\' + [IO.Path]::GetFileName($DriverInfPath))
    if (Test-Path -LiteralPath $windowsInf) {
        return (Resolve-Path -LiteralPath $windowsInf).Path
    }
    return ''
}

function Remove-InfComment([string]$Line) {
    if ($null -eq $Line) { return '' }
    $inQuote = $false
    for ($i = 0; $i -lt $Line.Length; $i++) {
        if ($Line[$i] -eq '"') { $inQuote = -not $inQuote }
        elseif ($Line[$i] -eq ';' -and -not $inQuote) { return $Line.Substring(0, $i) }
    }
    return $Line
}

function Resolve-InfToken([string]$Value, $Strings) {
    if ($null -eq $Value) { return '' }
    $trimmed = $Value.Trim().Trim('"')
    if ($trimmed -match '^%([^%]+)%$') {
        $key = $Matches[1].ToLowerInvariant()
        if ($Strings.ContainsKey($key)) {
            return ([string]$Strings[$key]).Trim().Trim('"')
        }
    }
    return $trimmed
}

function Get-InfInterfaceEvidence([string]$InfPath, [string]$DriverSection) {
    if ([string]::IsNullOrWhiteSpace($InfPath) -or -not (Test-Path -LiteralPath $InfPath)) {
        return @()
    }

    $lines = @(Get-Content -LiteralPath $InfPath -ErrorAction Stop)
    $strings = @{}
    $section = ''
    foreach ($raw in $lines) {
        $line = (Remove-InfComment $raw).Trim()
        if (-not $line) { continue }
        if ($line -match '^\[([^\]]+)\]$') {
            $section = $Matches[1].Trim()
            continue
        }
        if ($section -match '^(?i:Strings)(?:\..+)?$' -and $line -match '^([^=]+?)\s*=\s*(.+)$') {
            $key = $Matches[1].Trim().ToLowerInvariant()
            $value = $Matches[2].Trim().Trim('"')
            $strings[$key] = $value
        }
    }

    $audioCategory = '{6994AD04-93EF-11D0-A3CC-00A0C9223196}'
    $topologyCategory = '{DDA54A40-1E4C-11D1-A050-405705C10000}'
    $evidence = New-Object System.Collections.Generic.List[object]
    $section = ''
    foreach ($raw in $lines) {
        $line = (Remove-InfComment $raw).Trim()
        if (-not $line) { continue }
        if ($line -match '^\[([^\]]+)\]$') {
            $section = $Matches[1].Trim()
            continue
        }
        if ($line -notmatch '^(?i:AddInterface)\s*=\s*(.+)$') { continue }

        $parts = @($Matches[1] -split ',', 3)
        if ($parts.Count -lt 3) { continue }

        $categoryToken = $parts[0].Trim()
        $referenceToken = $parts[1].Trim()
        $installToken = $parts[2].Trim()
        $categoryResolved = Resolve-InfToken $categoryToken $strings
        $referenceResolved = Resolve-InfToken $referenceToken $strings
        $installResolved = Resolve-InfToken $installToken $strings

        $sectionRelevant = $true
        if (-not [string]::IsNullOrWhiteSpace($DriverSection)) {
            $prefix = $DriverSection + '.'
            $sectionRelevant = $section.Equals(($DriverSection + '.Interfaces'), [StringComparison]::OrdinalIgnoreCase) -or
                $section.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)
        }

        $evidence.Add([ordered]@{
            Section = $section
            SectionRelevant = [bool]$sectionRelevant
            CategoryToken = $categoryToken
            CategoryResolved = $categoryResolved
            ReferenceToken = $referenceToken
            ReferenceResolved = $referenceResolved
            InstallSectionToken = $installToken
            InstallSectionResolved = $installResolved
            IsAudio = $categoryResolved.Equals($audioCategory, [StringComparison]::OrdinalIgnoreCase)
            IsTopology = $categoryResolved.Equals($topologyCategory, [StringComparison]::OrdinalIgnoreCase)
        })
    }

    $relevant = @($evidence | Where-Object { $_.SectionRelevant })
    if ($relevant.Count -gt 0) { return $relevant }
    return @($evidence)
}

$defaultLines = @(& $EndpointCtl get-default 2>&1 | ForEach-Object { "$_" })
if ($LASTEXITCODE -ne 0) {
    throw "Could not resolve the current default endpoint: $($defaultLines -join ' | ')"
}
$defaultLine = $defaultLines | Where-Object { $_.StartsWith("DEFAULT`t") } | Select-Object -First 1
if (-not $defaultLine) {
    throw "Endpoint helper returned no DEFAULT record: $($defaultLines -join ' | ')"
}
$parts = $defaultLine -split "`t", 3
$mmDeviceId = $parts[1]
$friendlyName = $parts[2]

$audioEndpoints = @(Get-PnpDevice -Class AudioEndpoint -PresentOnly -ErrorAction Stop)
$endpointNode = $audioEndpoints | Where-Object {
    $_.InstanceId.EndsWith($mmDeviceId, [StringComparison]::OrdinalIgnoreCase)
} | Select-Object -First 1

if (-not $endpointNode) {
    # Some Windows builds expose a slightly different prefix. Preserve strict
    # GUID-tail matching as a fallback instead of guessing by friendly name.
    $guidTail = if ($mmDeviceId -match '(\{[0-9A-Fa-f-]{36}\})$') { $Matches[1] } else { '' }
    if ($guidTail) {
        $endpointNode = $audioEndpoints | Where-Object {
            $_.InstanceId.EndsWith($guidTail, [StringComparison]::OrdinalIgnoreCase)
        } | Select-Object -First 1
    }
}
if (-not $endpointNode) {
    throw "Could not map MMDevice '$mmDeviceId' to a present AudioEndpoint PnP node. No friendly-name fallback is used because that can select the wrong device."
}

$chain = New-Object System.Collections.Generic.List[object]
$currentId = [string]$endpointNode.InstanceId
$seen = @{}
for ($depth = 0; $depth -lt 10 -and -not [string]::IsNullOrWhiteSpace($currentId); $depth++) {
    if ($seen.ContainsKey($currentId)) { break }
    $seen[$currentId] = $true

    $node = Get-PnpDevice -InstanceId $currentId -ErrorAction SilentlyContinue
    $hardwareIds = @(Get-PnpPropertyData $currentId 'DEVPKEY_Device_HardwareIds') | Where-Object { $_ }
    $compatibleIds = @(Get-PnpPropertyData $currentId 'DEVPKEY_Device_CompatibleIds') | Where-Object { $_ }
    $parentId = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_Parent')
    $driverInfPath = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverInfPath')
    $driverInfSection = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverInfSection')
    $driverInfFullPath = Resolve-InstalledInfPath $driverInfPath
    $driverInterfaces = @(Get-InfInterfaceEvidence $driverInfFullPath $driverInfSection)
    $topologyReferences = @(
        $driverInterfaces |
            Where-Object { $_.IsTopology -and -not [string]::IsNullOrWhiteSpace($_.ReferenceResolved) } |
            ForEach-Object { [string]$_.ReferenceResolved } |
            Sort-Object -Unique
    )

    $chain.Add([ordered]@{
        Depth = $depth
        InstanceId = $currentId
        FriendlyName = if ($node) { [string]$node.FriendlyName } else { '' }
        Class = if ($node) { [string]$node.Class } else { '' }
        ClassGuid = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_ClassGuid')
        Manufacturer = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_Manufacturer')
        Service = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_Service')
        DriverInfPath = $driverInfPath
        DriverInfFullPath = $driverInfFullPath
        DriverInfSection = $driverInfSection
        DriverProvider = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverProvider')
        DriverVersion = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverVersion')
        HardwareIds = [string[]]$hardwareIds
        CompatibleIds = [string[]]$compatibleIds
        DriverInterfaces = @($driverInterfaces)
        TopologyReferenceCandidates = [string[]]$topologyReferences
        Parent = $parentId
    })

    $currentId = $parentId
}

$mediaClassGuid = '{4D36E96C-E325-11CE-BFC1-08002BE10318}'
$associationCandidates = @($chain | Where-Object {
    $_.HardwareIds.Count -gt 0 -and
    -not $_.InstanceId.StartsWith('SWD\MMDEVAPI\', [StringComparison]::OrdinalIgnoreCase) -and
    ($_.Class.Equals('MEDIA', [StringComparison]::OrdinalIgnoreCase) -or
     $_.ClassGuid.Equals($mediaClassGuid, [StringComparison]::OrdinalIgnoreCase))
})

$result = [ordered]@{
    Schema = 'omniphony.windows.apo-target.v2'
    CapturedAtUtc = [DateTime]::UtcNow.ToString('o')
    DefaultEndpoint = [ordered]@{
        MmDeviceId = $mmDeviceId
        FriendlyName = $friendlyName
        PnpInstanceId = [string]$endpointNode.InstanceId
    }
    AssociationCandidates = $associationCandidates
    PnpAncestry = @($chain)
}

$result | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

Write-Host "OMNIPHONY_APO_TARGET_CAPTURE_OK`t$OutputPath"
Write-Host "DEFAULT_ENDPOINT`t$friendlyName`t$mmDeviceId"
foreach ($candidate in $associationCandidates) {
    Write-Host "DRIVER_CANDIDATE`t$($candidate.InstanceId)`t$($candidate.DriverInfPath)`t$($candidate.DriverInfSection)`t$($candidate.Service)"
    foreach ($hardwareId in $candidate.HardwareIds) {
        Write-Host "HARDWARE_ID`t$hardwareId"
    }
    foreach ($topologyReference in $candidate.TopologyReferenceCandidates) {
        Write-Host "TOPOLOGY_REFERENCE`t$topologyReference"
    }
}
if ($associationCandidates.Count -eq 0) {
    Write-Warning 'No MEDIA-class parent with hardware IDs was found. The ancestry is preserved for diagnosis; do not invent an extension-INF target.'
}
