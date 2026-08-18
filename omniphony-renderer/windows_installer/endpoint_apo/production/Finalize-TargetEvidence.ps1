param(
    [Parameter(Mandatory = $true)]
    [string]$InputJson,

    [Parameter(Mandatory = $true)]
    [string]$OutputJson,

    [string]$InfRoot = ''
)

$ErrorActionPreference = 'Stop'
$audioCategory = '{6994AD04-93EF-11D0-A3CC-00A0C9223196}'
$topologyCategory = '{DDA54A40-1E4C-11D1-A050-405705C10000}'

function Remove-InfComment([string]$Line) {
    if ($null -eq $Line) { return '' }
    $inQuote = $false
    for ($i = 0; $i -lt $Line.Length; $i++) {
        if ($Line[$i] -eq '"') { $inQuote = -not $inQuote }
        elseif ($Line[$i] -eq ';' -and -not $inQuote) { return $Line.Substring(0, $i) }
    }
    return $Line
}

function Read-InfDocument([string]$Path) {
    $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
    $sections = @{}
    $strings = @{}
    $section = ''
    foreach ($raw in @(Get-Content -LiteralPath $resolved -ErrorAction Stop)) {
        $line = (Remove-InfComment ([string]$raw)).Trim()
        if (-not $line) { continue }
        if ($line -match '^\[([^\]]+)\]$') {
            $section = $Matches[1].Trim()
            $key = $section.ToLowerInvariant()
            if (-not $sections.ContainsKey($key)) {
                $sections[$key] = [pscustomobject]@{
                    DisplayName = $section
                    Lines = New-Object System.Collections.Generic.List[string]
                }
            }
            continue
        }
        if (-not $section) { continue }
        $sectionKey = $section.ToLowerInvariant()
        $sections[$sectionKey].Lines.Add($line)
        if ($section -match '^(?i:Strings)(?:\..+)?$' -and $line -match '^([^=]+?)\s*=\s*(.+)$') {
            $strings[$Matches[1].Trim().ToLowerInvariant()] = $Matches[2].Trim().Trim('"')
        }
    }
    return [pscustomobject]@{
        Path = $resolved
        Name = [IO.Path]::GetFileName($resolved)
        Sections = $sections
        Strings = $strings
    }
}

function Resolve-InfToken([string]$Value, $Document) {
    if ($null -eq $Value) { return '' }
    $trimmed = $Value.Trim().Trim('"')
    if ($trimmed -match '^%([^%]+)%$') {
        $key = $Matches[1].ToLowerInvariant()
        if ($Document.Strings.ContainsKey($key)) { return [string]$Document.Strings[$key] }
    }
    return $trimmed
}

function Resolve-ExactOrUniqueDecoratedSection($Document, [string]$Requested) {
    $key = $Requested.ToLowerInvariant()
    if ($Document.Sections.ContainsKey($key)) { return [string]$Document.Sections[$key].DisplayName }
    $prefix = $key + '.'
    $matches = @($Document.Sections.Keys | Where-Object { $_.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) } | Sort-Object)
    if ($matches.Count -eq 1) { return [string]$Document.Sections[$matches[0]].DisplayName }
    return ''
}

function Resolve-IncludedInfPath([string]$ParentPath, [string]$Token, $Document, [string]$ExplicitInfRoot) {
    $name = (Resolve-InfToken $Token $Document).Trim().Trim('"')
    if (-not $name) { return '' }
    if ([IO.Path]::IsPathRooted($name) -and (Test-Path -LiteralPath $name -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $name).Path
    }
    $beside = Join-Path ([IO.Path]::GetDirectoryName($ParentPath)) ([IO.Path]::GetFileName($name))
    if (Test-Path -LiteralPath $beside -PathType Leaf) { return (Resolve-Path -LiteralPath $beside).Path }
    if (-not [string]::IsNullOrWhiteSpace($ExplicitInfRoot)) {
        $rooted = Join-Path $ExplicitInfRoot ([IO.Path]::GetFileName($name))
        if (Test-Path -LiteralPath $rooted -PathType Leaf) { return (Resolve-Path -LiteralPath $rooted).Path }
    }
    $windowsInf = Join-Path $env:WINDIR ('INF\' + [IO.Path]::GetFileName($name))
    if (Test-Path -LiteralPath $windowsInf -PathType Leaf) { return (Resolve-Path -LiteralPath $windowsInf).Path }
    return ''
}

function Get-ResolvedInstallSection([string]$Base, [string]$Extension) {
    $baseTrimmed = $Base.Trim()
    $ext = $Extension.Trim()
    if (-not $baseTrimmed) { return '' }
    if (-not $ext) { return $baseTrimmed }
    if (-not $ext.StartsWith('.')) { $ext = '.' + $ext }
    if ($baseTrimmed.EndsWith($ext, [StringComparison]::OrdinalIgnoreCase)) { return $baseTrimmed }
    return $baseTrimmed + $ext
}

function Get-InterfaceEvidence([string]$PrimaryInf, [string]$InstallSection, [string]$ExplicitInfRoot) {
    $documents = @{}
    $visited = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $evidence = New-Object System.Collections.Generic.List[object]
    $warnings = New-Object System.Collections.Generic.List[string]
    $visitedRows = New-Object System.Collections.Generic.List[object]

    function Get-Document([string]$Path) {
        $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
        $key = $resolved.ToLowerInvariant()
        if (-not $documents.ContainsKey($key)) { $documents[$key] = Read-InfDocument $resolved }
        return $documents[$key]
    }

    function Visit([string]$Path, [string]$RequestedSection, [string]$Via, [int]$Depth) {
        if ($Depth -gt 12) {
            $warnings.Add("INF Include/Needs traversal exceeded depth 12 at $Path [$RequestedSection]")
            return
        }
        $document = Get-Document $Path
        $resolvedSection = Resolve-ExactOrUniqueDecoratedSection $document $RequestedSection
        if (-not $resolvedSection) {
            $warnings.Add("INF section was not found unambiguously: $($document.Name) [$RequestedSection]")
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

        $includes = New-Object System.Collections.Generic.List[string]
        $needs = New-Object System.Collections.Generic.List[string]
        $sectionKey = $resolvedSection.ToLowerInvariant()
        foreach ($line in @($document.Sections[$sectionKey].Lines)) {
            if ($line -notmatch '^([^=]+?)\s*=\s*(.*)$') { continue }
            $directive = $Matches[1].Trim().ToLowerInvariant()
            $value = $Matches[2].Trim()
            if ($directive -eq 'addinterface') {
                $parts = @($value -split ',', 4)
                if ($parts.Count -lt 2) { continue }
                $categoryToken = $parts[0].Trim()
                $referenceToken = $parts[1].Trim()
                $installToken = if ($parts.Count -ge 3) { $parts[2].Trim() } else { '' }
                $categoryResolved = Resolve-InfToken $categoryToken $document
                $referenceResolved = Resolve-InfToken $referenceToken $document
                $installResolved = Resolve-InfToken $installToken $document
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
            } elseif ($directive -eq 'include') {
                foreach ($token in @($value -split ',')) { if ($token.Trim()) { $includes.Add($token.Trim()) } }
            } elseif ($directive -eq 'needs') {
                foreach ($token in @($value -split ',')) { if ($token.Trim()) { $needs.Add($token.Trim()) } }
            }
        }

        if ($needs.Count -eq 0) { return }
        if ($includes.Count -eq 0) {
            $warnings.Add("$($document.Name) [$resolvedSection] has Needs= without Include=; refusing cross-INF guessing.")
            return
        }

        $includedDocuments = New-Object System.Collections.Generic.List[object]
        foreach ($include in $includes) {
            $path = Resolve-IncludedInfPath $document.Path $include $document $ExplicitInfRoot
            if (-not $path) {
                $warnings.Add("Included INF not found from $($document.Name) [$resolvedSection]: $include")
                continue
            }
            try { $includedDocuments.Add((Get-Document $path)) }
            catch { $warnings.Add("Could not parse included INF '$path': $($_.Exception.Message)") }
        }

        foreach ($needToken in $needs) {
            $need = Resolve-InfToken $needToken $document
            $matches = New-Object System.Collections.Generic.List[object]
            foreach ($included in $includedDocuments) {
                $match = Resolve-ExactOrUniqueDecoratedSection $included $need
                if ($match) { $matches.Add([pscustomobject]@{ Document = $included; Section = $match }) }
            }
            if ($matches.Count -eq 0) {
                $warnings.Add("Needs section not found in included INFs from $($document.Name) [$resolvedSection]: $need")
                continue
            }
            if ($matches.Count -gt 1) {
                $locations = @($matches | ForEach-Object { "$($_.Document.Name)[$($_.Section)]" }) -join ', '
                $warnings.Add("Needs section is ambiguous across included INFs: $need -> $locations")
                continue
            }
            Visit $matches[0].Document.Path $matches[0].Section "Include/Needs from $($document.Name)[$resolvedSection]" ($Depth + 1)
        }
    }

    if (-not (Test-Path -LiteralPath $PrimaryInf -PathType Leaf)) {
        return [pscustomobject]@{
            Evidence = @()
            Warnings = @("installed INF is not readable: $PrimaryInf")
            VisitedSections = @()
        }
    }
    if (-not $InstallSection) {
        return [pscustomobject]@{ Evidence = @(); Warnings = @('resolved installed INF section is empty'); VisitedSections = @() }
    }
    Visit $PrimaryInf ($InstallSection + '.Interfaces') 'installed-driver-section+platform-extension' 0
    return [pscustomobject]@{
        Evidence = @($evidence)
        Warnings = @($warnings | Sort-Object -Unique)
        VisitedSections = @($visitedRows)
    }
}

function Get-PairedTopologyReferences($Interfaces) {
    $references = @{}
    $display = @{}
    foreach ($item in @($Interfaces)) {
        $ref = ([string]$item.ReferenceResolved).Trim()
        $category = ([string]$item.CategoryResolved).Trim().ToLowerInvariant()
        if (-not $ref -or -not $category) { continue }
        $key = $ref.ToLowerInvariant()
        if (-not $references.ContainsKey($key)) { $references[$key] = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase) }
        $null = $references[$key].Add($category)
        if (-not $display.ContainsKey($key)) { $display[$key] = $ref }
    }
    return @($references.Keys | Where-Object {
        $references[$_].Contains($audioCategory) -and $references[$_].Contains($topologyCategory)
    } | ForEach-Object { $display[$_] } | Sort-Object)
}

$inputPath = (Resolve-Path -LiteralPath $InputJson -ErrorAction Stop).Path
$capture = Get-Content -LiteralPath $inputPath -Raw | ConvertFrom-Json
if ([string]$capture.Schema -ne 'omniphony.windows.apo-target.v2') {
    throw "Expected low-level omniphony.windows.apo-target.v2 input, got '$($capture.Schema)'."
}

foreach ($candidate in @($capture.AssociationCandidates)) {
    $baseSection = [string]$candidate.DriverInfSection
    $sectionExt = [string]$candidate.DriverInfSectionExt
    $resolvedSection = Get-ResolvedInstallSection $baseSection $sectionExt
    $infPath = [string]$candidate.DriverInfFullPath
    if ((-not (Test-Path -LiteralPath $infPath -PathType Leaf)) -and -not [string]::IsNullOrWhiteSpace($InfRoot)) {
        $alternate = Join-Path $InfRoot ([IO.Path]::GetFileName($infPath))
        if (Test-Path -LiteralPath $alternate -PathType Leaf) { $infPath = (Resolve-Path -LiteralPath $alternate).Path }
    }
    $result = Get-InterfaceEvidence $infPath $resolvedSection $InfRoot
    $interfaces = @($result.Evidence)
    $topologyRefs = @($interfaces | Where-Object { $_.IsTopology -and $_.ReferenceResolved } | ForEach-Object { [string]$_.ReferenceResolved } | Sort-Object -Unique)
    $pairedRefs = @(Get-PairedTopologyReferences $interfaces)

    $candidate | Add-Member -NotePropertyName DriverInfSectionBase -NotePropertyValue $baseSection -Force
    $candidate | Add-Member -NotePropertyName DriverInfSectionExt -NotePropertyValue $sectionExt -Force
    $candidate | Add-Member -NotePropertyName DriverInfResolvedSection -NotePropertyValue $resolvedSection -Force
    $candidate | Add-Member -NotePropertyName DriverInterfaces -NotePropertyValue $interfaces -Force
    $candidate | Add-Member -NotePropertyName InterfaceResolutionWarnings -NotePropertyValue @($result.Warnings) -Force
    $candidate | Add-Member -NotePropertyName InterfaceResolutionVisitedSections -NotePropertyValue @($result.VisitedSections) -Force
    $candidate | Add-Member -NotePropertyName TopologyReferenceCandidates -NotePropertyValue $topologyRefs -Force
    $candidate | Add-Member -NotePropertyName PairedTopologyReferenceCandidates -NotePropertyValue $pairedRefs -Force
}

$capture.Schema = 'omniphony.windows.apo-target.v3'
$capture | Add-Member -NotePropertyName EvidenceFinalizer -NotePropertyValue 'Finalize-TargetEvidence.ps1' -Force
$outputFull = [IO.Path]::GetFullPath($OutputJson)
$outputParent = Split-Path -Parent $outputFull
if ($outputParent) { New-Item -ItemType Directory -Force -Path $outputParent | Out-Null }
$capture | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $outputFull -Encoding UTF8
Write-Host "OMNIPHONY_TARGET_EVIDENCE_V3_OK`t$outputFull"
