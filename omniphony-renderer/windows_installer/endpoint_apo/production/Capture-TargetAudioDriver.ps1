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

function Read-InfDocument([string]$InfPath) {
    $resolved = (Resolve-Path -LiteralPath $InfPath -ErrorAction Stop).Path
    $sections = @{}
    $strings = @{}
    $section = ''

    foreach ($raw in @(Get-Content -LiteralPath $resolved -ErrorAction Stop)) {
        $line = (Remove-InfComment $raw).Trim()
        if (-not $line) { continue }
        if ($line -match '^\[([^\]]+)\]$') {
            $section = $Matches[1].Trim()
            if (-not $sections.ContainsKey($section)) {
                $sections[$section] = New-Object System.Collections.Generic.List[string]
            }
            continue
        }
        if (-not $section) { continue }
        $sections[$section].Add($line)
        if ($section -match '^(?i:Strings)(?:\..+)?$' -and $line -match '^([^=]+?)\s*=\s*(.+)$') {
            $key = $Matches[1].Trim().ToLowerInvariant()
            $strings[$key] = $Matches[2].Trim().Trim('"')
        }
    }

    return [pscustomobject]@{
        Path = $resolved
        Name = [IO.Path]::GetFileName($resolved)
        Sections = $sections
        Strings = $strings
    }
}

function Resolve-IncludedInfPath([string]$ParentInfPath, [string]$IncludeToken, $Strings) {
    $name = Resolve-InfToken $IncludeToken $Strings
    if ([string]::IsNullOrWhiteSpace($name)) { return '' }
    $name = $name.Trim().Trim('"')
    if ([IO.Path]::IsPathRooted($name)) {
        if (Test-Path -LiteralPath $name -PathType Leaf) { return (Resolve-Path -LiteralPath $name).Path }
        return ''
    }

    $beside = Join-Path ([IO.Path]::GetDirectoryName($ParentInfPath)) $name
    if (Test-Path -LiteralPath $beside -PathType Leaf) { return (Resolve-Path -LiteralPath $beside).Path }
    $windowsInf = Join-Path $env:WINDIR ('INF\' + [IO.Path]::GetFileName($name))
    if (Test-Path -LiteralPath $windowsInf -PathType Leaf) { return (Resolve-Path -LiteralPath $windowsInf).Path }
    return ''
}

function Get-InfInterfaceEvidence([string]$InfPath, [string]$DriverSection) {
    $empty = [pscustomobject]@{ Evidence = @(); Warnings = @(); VisitedSections = @() }
    if ([string]::IsNullOrWhiteSpace($InfPath) -or -not (Test-Path -LiteralPath $InfPath -PathType Leaf)) {
        return $empty
    }
    if ([string]::IsNullOrWhiteSpace($DriverSection)) {
        return [pscustomobject]@{
            Evidence = @()
            Warnings = @('DriverInfSection is empty; refusing to scan unrelated AddInterface declarations.')
            VisitedSections = @()
        }
    }

    $audioCategory = '{6994AD04-93EF-11D0-A3CC-00A0C9223196}'
    $topologyCategory = '{DDA54A40-1E4C-11D1-A050-405705C10000}'
    $documents = @{}
    $evidence = New-Object System.Collections.Generic.List[object]
    $warnings = New-Object System.Collections.Generic.List[string]
    $visitedRows = New-Object System.Collections.Generic.List[object]
    $visited = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)

    function Get-CachedInfDocument([string]$Path) {
        $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
        $key = $resolved.ToLowerInvariant()
        if (-not $documents.ContainsKey($key)) { $documents[$key] = Read-InfDocument $resolved }
        return $documents[$key]
    }

    function Get-ExactOrUniqueDecoratedSection($Document, [string]$Requested) {
        if ($Document.Sections.ContainsKey($Requested)) { return $Requested }
        $prefix = $Requested + '.'
        $matches = @($Document.Sections.Keys | Where-Object {
            $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)
        } | Sort-Object)
        if ($matches.Count -eq 1) { return [string]$matches[0] }
        return ''
    }

    function Visit-InfSection([string]$Path, [string]$SectionName, [string]$Via, [int]$Depth) {
        if ($Depth -gt 12) {
            $warnings.Add("INF Include/Needs traversal exceeded depth 12 at $Path [$SectionName]")
            return
        }
        $document = Get-CachedInfDocument $Path
        $resolvedSection = Get-ExactOrUniqueDecoratedSection $document $SectionName
        if (-not $resolvedSection) {
            $warnings.Add("INF section was not found unambiguously: $($document.Name) [$SectionName]")
            return
        }

        $visitKey = "$($document.Path)|$resolvedSection"
        if (-not $visited.Add($visitKey)) { return }
        $visitedRows.Add([ordered]@{
            InfPath = $document.Path
            InfName = $document.Name
            Section = $resolvedSection
            Via = $Via
            Depth = $Depth
        })

        $includeTokens = New-Object System.Collections.Generic.List[string]
        $needsTokens = New-Object System.Collections.Generic.List[string]

        foreach ($line in @($document.Sections[$resolvedSection])) {
            if ($line -match '^(?i:AddInterface)\s*=\s*(.+)$') {
                $parts = @($Matches[1] -split ',', 4)
                if ($parts.Count -lt 2) { continue }
                $categoryToken = $parts[0].Trim()
                $referenceToken = $parts[1].Trim()
                $installToken = if ($parts.Count -ge 3) { $parts[2].Trim() } else { '' }
                $categoryResolved = Resolve-InfToken $categoryToken $document.Strings
                $referenceResolved = Resolve-InfToken $referenceToken $document.Strings
                $installResolved = Resolve-InfToken $installToken $document.Strings
                $evidence.Add([ordered]@{
                    Section = $resolvedSection
                    SectionRelevant = $true
                    SourceInfPath = $document.Path
                    SourceInfName = $document.Name
                    ResolutionVia = $Via
                    ResolutionDepth = $Depth
                    CategoryToken = $categoryToken
                    CategoryResolved = $categoryResolved
                    ReferenceToken = $referenceToken
                    ReferenceResolved = $referenceResolved
                    InstallSectionToken = $installToken
                    InstallSectionResolved = $installResolved
                    IsAudio = $categoryResolved.Equals($audioCategory, [StringComparison]::OrdinalIgnoreCase)
                    IsTopology = $categoryResolved.Equals($topologyCategory, [StringComparison]::OrdinalIgnoreCase)
                })
                continue
            }
            if ($line -match '^(?i:Include)\s*=\s*(.+)$') {
                foreach ($token in @($Matches[1] -split ',')) {
                    if (-not [string]::IsNullOrWhiteSpace($token)) { $includeTokens.Add($token.Trim()) }
                }
                continue
            }
            if ($line -match '^(?i:Needs)\s*=\s*(.+)$') {
                foreach ($token in @($Matches[1] -split ',')) {
                    if (-not [string]::IsNullOrWhiteSpace($token)) { $needsTokens.Add($token.Trim()) }
                }
            }
        }

        if ($needsTokens.Count -eq 0) { return }
        if ($includeTokens.Count -eq 0) {
            $warnings.Add("$($document.Name) [$resolvedSection] has Needs= without Include=; refusing cross-INF guessing.")
            return
        }

        $includedDocs = New-Object System.Collections.Generic.List[object]
        foreach ($includeToken in $includeTokens) {
            $includedPath = Resolve-IncludedInfPath $document.Path $includeToken $document.Strings
            if (-not $includedPath) {
                $warnings.Add("Included INF not found from $($document.Name) [$resolvedSection]: $includeToken")
                continue
            }
            try { $includedDocs.Add((Get-CachedInfDocument $includedPath)) }
            catch { $warnings.Add("Could not parse included INF '$includedPath': $($_.Exception.Message)") }
        }

        foreach ($needToken in $needsTokens) {
            $need = Resolve-InfToken $needToken $document.Strings
            if ([string]::IsNullOrWhiteSpace($need)) { continue }
            $matches = New-Object System.Collections.Generic.List[object]
            foreach ($included in $includedDocs) {
                $match = Get-ExactOrUniqueDecoratedSection $included $need
                if ($match) {
                    $matches.Add([pscustomobject]@{ Document = $included; Section = $match })
                }
            }
            if ($matches.Count -eq 0) {
                $warnings.Add("Needs section not found in included INFs from $($document.Name) [$resolvedSection]: $need")
                continue
            }
            if ($matches.Count -gt 1) {
                $where = @($matches | ForEach-Object { "$($_.Document.Name)[$($_.Section)]" }) -join ', '
                $warnings.Add("Needs section is ambiguous across included INFs: $need -> $where")
                continue
            }
            $target = $matches[0]
            Visit-InfSection $target.Document.Path $target.Section "Include/Needs from $($document.Name)[$resolvedSection]" ($Depth + 1)
        }
    }

    $primary = Get-CachedInfDocument $InfPath
    $requested = if ($DriverSection.EndsWith('.Interfaces', [StringComparison]::OrdinalIgnoreCase)) {
        $DriverSection
    } else {
        $DriverSection + '.Interfaces'
    }
    $initial = Get-ExactOrUniqueDecoratedSection $primary $requested
    if (-not $initial) {
        $warnings.Add("No unambiguous interfaces section matched installed driver section '$DriverSection' in $($primary.Name). Refusing whole-INF AddInterface fallback.")
    } else {
        Visit-InfSection $primary.Path $initial 'installed-driver-section' 0
    }

    return [pscustomobject]@{
        Evidence = @($evidence)
        Warnings = @($warnings | Sort-Object -Unique)
        VisitedSections = @($visitedRows)
    }
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
    $interfaceResolution = Get-InfInterfaceEvidence $driverInfFullPath $driverInfSection
    $driverInterfaces = @($interfaceResolution.Evidence)
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
        InterfaceResolutionWarnings = @($interfaceResolution.Warnings)
        InterfaceResolutionVisitedSections = @($interfaceResolution.VisitedSections)
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

$result | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

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
    foreach ($warning in $candidate.InterfaceResolutionWarnings) {
        Write-Warning "INF_INTERFACE_EVIDENCE $warning"
    }
}
if ($associationCandidates.Count -eq 0) {
    Write-Warning 'No MEDIA-class parent with hardware IDs was found. The ancestry is preserved for diagnosis; do not invent an extension-INF target.'
}
