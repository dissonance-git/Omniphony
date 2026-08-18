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
    throw 'Run the production Omniphony uninstaller from an elevated PowerShell session.'
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
            }
        }
    } | Where-Object { $_ -and $_.PublishedName })
}

function Invoke-PnpUtil([string[]]$Arguments, [string]$Label) {
    Write-Host "PNPUTIL $Label"
    & "$env:WINDIR\System32\pnputil.exe" @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Label failed with pnputil exit code $LASTEXITCODE" }
}

$packages = @(Get-OmniphonyPackages | Sort-Object @{ Expression = { if ($_.OriginalName -ieq 'OmniphonyApoExtension.inf') { 0 } else { 1 } } })
foreach ($package in $packages) {
    Invoke-PnpUtil @('/delete-driver', $package.PublishedName, '/uninstall', '/force') "remove $($package.OriginalName) [$($package.PublishedName)]"
}
Invoke-PnpUtil @('/scan-devices') 'rescan devices'

$service = Get-Service -Name AudioSrv -ErrorAction Stop
if ($service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
Start-Service -Name AudioSrv
Start-Sleep -Milliseconds 800

$remaining = @(Get-OmniphonyPackages)
if ($remaining.Count -ne 0) {
    throw "Omniphony production packages remain in DriverStore: $($remaining.PublishedName -join ', ')"
}

$statePath = Join-Path $StateRoot 'installed-packages.json'
if (Test-Path -LiteralPath $statePath) { Remove-Item -LiteralPath $statePath -Force }

Write-Host 'OMNIPHONY_PRODUCTION_UNINSTALL_OK 1'
Write-Host 'Physical audio driver was not removed or modified.'
Write-Host 'DisableProtectedAudioDG was not modified.'
