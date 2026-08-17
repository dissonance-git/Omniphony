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

    $chain.Add([ordered]@{
        Depth = $depth
        InstanceId = $currentId
        FriendlyName = if ($node) { [string]$node.FriendlyName } else { '' }
        Class = if ($node) { [string]$node.Class } else { '' }
        ClassGuid = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_ClassGuid')
        Manufacturer = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_Manufacturer')
        Service = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_Service')
        DriverInfPath = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverInfPath')
        DriverProvider = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverProvider')
        DriverVersion = [string](Get-PnpPropertyData $currentId 'DEVPKEY_Device_DriverVersion')
        HardwareIds = [string[]]$hardwareIds
        CompatibleIds = [string[]]$compatibleIds
        Parent = $parentId
    })

    $currentId = $parentId
}

$associationCandidates = @($chain | Where-Object {
    $_.HardwareIds.Count -gt 0 -and -not $_.InstanceId.StartsWith('SWD\MMDEVAPI\', [StringComparison]::OrdinalIgnoreCase)
})

$result = [ordered]@{
    Schema = 'omniphony.windows.apo-target.v1'
    CapturedAtUtc = [DateTime]::UtcNow.ToString('o')
    DefaultEndpoint = [ordered]@{
        MmDeviceId = $mmDeviceId
        FriendlyName = $friendlyName
        PnpInstanceId = [string]$endpointNode.InstanceId
    }
    AssociationCandidates = $associationCandidates
    PnpAncestry = @($chain)
}

$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

Write-Host "OMNIPHONY_APO_TARGET_CAPTURE_OK`t$OutputPath"
Write-Host "DEFAULT_ENDPOINT`t$friendlyName`t$mmDeviceId"
foreach ($candidate in $associationCandidates) {
    Write-Host "DRIVER_CANDIDATE`t$($candidate.InstanceId)`t$($candidate.DriverInfPath)`t$($candidate.Service)"
    foreach ($hardwareId in $candidate.HardwareIds) {
        Write-Host "HARDWARE_ID`t$hardwareId"
    }
}
if ($associationCandidates.Count -eq 0) {
    Write-Warning 'No parent with hardware IDs was found. The JSON ancestry is still useful evidence; do not invent an extension-INF target.'
}
