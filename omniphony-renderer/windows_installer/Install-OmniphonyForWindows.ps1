param(
    [ValidateSet('Install', 'Uninstall', 'Validate')]
    [string]$Action = 'Install',

    [Parameter(Mandatory = $true)]
    [string]$AppRoot,

    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$StateRoot = Join-Path $env:ProgramData 'Omniphony'
$LogPath = Join-Path $StateRoot 'installer.log'
$TransportStatePath = Join-Path $StateRoot 'development-transport.txt'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$CurrentEndpointServiceName = 'VirtualAudioDriver'
$EndpointCtlPath = Join-Path $AppRoot 'support\OmniphonyEndpointCtl.exe'
$SteamDriverInf = if (${env:CommonProgramFiles(x86)}) {
    Join-Path ${env:CommonProgramFiles(x86)} 'Steam\drivers\Windows10\x64\SteamStreamingSpeakers.inf'
} else {
    $null
}

function Write-InstallLog([string]$Message) {
    New-Item -ItemType Directory -Force -Path $StateRoot | Out-Null
    Add-Content -LiteralPath $LogPath -Value "[$(Get-Date -Format o)] $Message" -Encoding utf8
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Require-Administrator {
    if (-not (Test-Administrator)) {
        throw 'Omniphony for Windows installation requires administrator elevation.'
    }
}

function Test-LegacyOmniphonyService($Service) {
    if (-not $Service) { return $false }
    if ($Service.Name -eq $CurrentEndpointServiceName) { return $false }

    $name = [string]$Service.Name
    $display = [string]$Service.DisplayName
    return $name -match '(?i)^Omniphony' -or
           $display -match '(?i)^Omniphony' -or
           $name -ieq 'Spatial' -or
           $display -ieq 'Spatial'
}

function Stop-OldHosts {
    foreach ($name in @('Omniphony', 'Spatial')) {
        Get-Process -Name $name -ErrorAction SilentlyContinue |
            Stop-Process -Force -ErrorAction SilentlyContinue
    }

    $legacyServices = @(
        Get-CimInstance Win32_Service -ErrorAction SilentlyContinue |
            Where-Object { Test-LegacyOmniphonyService $_ }
    )
    foreach ($service in $legacyServices) {
        try {
            if ($service.State -ne 'Stopped') {
                & "$env:WINDIR\System32\sc.exe" stop $service.Name | Out-Null
            }
            & "$env:WINDIR\System32\sc.exe" config $service.Name start= disabled | Out-Null
            Write-InstallLog "Retired legacy audio service: $($service.Name) [$($service.DisplayName)]"
        }
        catch {
            Write-InstallLog "Legacy service retirement failed: $($service.Name) $($_.Exception.Message)"
        }
    }
}

function Remove-LegacyRunEntries {
    if (-not (Test-Path $RunKey)) { return }
    foreach ($name in @('Spatial', 'OmniphonyForHeadphones')) {
        Remove-ItemProperty -LiteralPath $RunKey -Name $name -ErrorAction SilentlyContinue
    }
}

function Remove-OmniphonyRunEntry {
    if (Test-Path $RunKey) {
        Remove-ItemProperty -LiteralPath $RunKey -Name 'Omniphony' -ErrorAction SilentlyContinue
    }
    Remove-LegacyRunEntries
}

function Invoke-EndpointCtlRaw([string[]]$Arguments) {
    if (-not (Test-Path -LiteralPath $EndpointCtlPath)) {
        throw "Omniphony endpoint control helper is missing: $EndpointCtlPath"
    }

    $raw = & $EndpointCtlPath @Arguments 2>&1
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 1 }
    $lines = @($raw | ForEach-Object { [string]$_ })
    foreach ($line in $lines) {
        if ($line) { Write-InstallLog "EndpointCtl: $line" }
    }

    return [pscustomobject]@{
        ExitCode = [int]$code
        Lines = $lines
    }
}

function Get-EndpointIdByFriendlyName([string[]]$Needles) {
    $result = Invoke-EndpointCtlRaw (@('find-name') + $Needles)
    if ($result.ExitCode -eq 3) { return $null }
    if ($result.ExitCode -ne 0) {
        throw "Endpoint enumeration failed (exit $($result.ExitCode)): $($result.Lines -join ' | ')"
    }

    $line = $result.Lines | Where-Object { $_ -like "ENDPOINT`t*" } | Select-Object -First 1
    if (-not $line) {
        throw 'Endpoint control helper returned success without an ENDPOINT record.'
    }
    $parts = $line -split "`t", 3
    if ($parts.Count -lt 2 -or -not $parts[1]) {
        throw "Endpoint control helper returned a malformed ENDPOINT record: $line"
    }
    return $parts[1]
}

function Wait-EndpointIdByFriendlyName([string[]]$Needles, [int]$Seconds = 20) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $id = Get-EndpointIdByFriendlyName $Needles
        if ($id) { return $id }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

function Set-DefaultEndpointByName([string[]]$Needles, [int]$Seconds = 20) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $result = Invoke-EndpointCtlRaw (@('set-default-name') + $Needles)
        if ($result.ExitCode -eq 0) {
            $line = $result.Lines | Where-Object { $_ -like "SET`t*" } | Select-Object -First 1
            if (-not $line) {
                throw 'Endpoint control helper returned success without a SET record.'
            }
            $parts = $line -split "`t", 3
            if ($parts.Count -lt 2 -or -not $parts[1]) {
                throw "Endpoint control helper returned a malformed SET record: $line"
            }
            Write-InstallLog "Default render endpoint set to $($Needles -join ' / ') [$($parts[1])]"
            return $parts[1]
        }
        if ($result.ExitCode -ne 3) {
            throw "Default endpoint switch failed (exit $($result.ExitCode)): $($result.Lines -join ' | ')"
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)

    throw "Windows did not expose an active render endpoint matching: $($Needles -join ', ')"
}

function Set-DefaultEndpointById([string]$EndpointId) {
    $result = Invoke-EndpointCtlRaw @('set-default-id', $EndpointId)
    if ($result.ExitCode -ne 0) {
        throw "Default endpoint restore failed (exit $($result.ExitCode)): $($result.Lines -join ' | ')"
    }
}

function Import-DevelopmentCertificate([string]$CertificatePath) {
    if (-not (Test-Path $CertificatePath)) { throw "Development certificate missing: $CertificatePath" }
    $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($CertificatePath)
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPublisher' | Out-Null
    Write-InstallLog "Imported development driver certificate $($cert.Thumbprint)"
    return $cert.Thumbprint
}

function Remove-DevelopmentCertificate([string]$CertificatePath) {
    if (-not (Test-Path $CertificatePath)) { return }
    $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($CertificatePath)
    $removed = $false
    foreach ($store in @('Root', 'TrustedPublisher')) {
        $path = "Cert:\LocalMachine\$store\$($cert.Thumbprint)"
        if (Test-Path $path) {
            Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
            $removed = $true
        }
    }
    if ($removed) {
        Write-InstallLog "Removed development driver certificate $($cert.Thumbprint)"
    }
}

function Set-TransportState([string]$Transport) {
    New-Item -ItemType Directory -Force -Path $StateRoot | Out-Null
    Set-Content -LiteralPath $TransportStatePath -Value $Transport -Encoding ascii
    Write-InstallLog "Development transport: $Transport"
}

function Remove-TransportState {
    Remove-Item -LiteralPath $TransportStatePath -Force -ErrorAction SilentlyContinue
}

function Install-SteamTransport {
    $needles = @('Steam Streaming Speakers')
    $existing = Get-EndpointIdByFriendlyName $needles
    if ($existing) {
        Write-InstallLog "Using existing Steam Streaming Speakers endpoint [$existing]"
        Set-DefaultEndpointByName $needles | Out-Null
        Set-TransportState 'steam-streaming-speakers'
        return
    }

    if (-not $SteamDriverInf -or -not (Test-Path -LiteralPath $SteamDriverInf)) {
        throw 'Steam Streaming Speakers is not installed and the official Steam-local driver package was not found.'
    }

    $resolvedInf = (Resolve-Path -LiteralPath $SteamDriverInf).Path
    Write-InstallLog "Installing official Steam-local virtual sink: $resolvedInf"
    $installResult = Invoke-EndpointCtlRaw @('install-driver', $resolvedInf)
    if ($installResult.ExitCode -ne 0) {
        throw "Steam Streaming Speakers driver installation failed (exit $($installResult.ExitCode)): $($installResult.Lines -join ' | ')"
    }
    $needsReboot = [bool]($installResult.Lines | Where-Object { $_ -like '*REBOOT=1*' } | Select-Object -First 1)
    if ($needsReboot) {
        Write-InstallLog 'Steam Streaming Speakers installation reported that Windows may require a reboot.'
    }

    $id = Wait-EndpointIdByFriendlyName $needles 20
    if (-not $id) {
        if ($needsReboot) {
            throw 'Steam Streaming Speakers installed but Windows requested a reboot before exposing the endpoint.'
        }
        throw 'Steam Streaming Speakers driver installation succeeded but Windows did not expose the render endpoint.'
    }

    Set-DefaultEndpointByName $needles | Out-Null
    Set-TransportState 'steam-streaming-speakers'
    Write-InstallLog 'Steam signed development transport is ready. Omniphony does not own or modify the Valve driver.'
}

function Install-DevelopmentOmniphonyEndpoint([string]$DriverScript, [string]$Certificate) {
    Import-DevelopmentCertificate $Certificate | Out-Null
    & $DriverScript -Action Install
    if ($LASTEXITCODE -ne 0) {
        throw "Development Omniphony endpoint installer failed: $LASTEXITCODE"
    }
    Set-DefaultEndpointByName @('Omniphony', 'Spatial') | Out-Null
    Set-TransportState 'omniphony-development-endpoint'
}

if ($Action -eq 'Validate') {
    $helperSource = Join-Path $PSScriptRoot 'OmniphonyEndpointCtl.cpp'
    $helperProject = Join-Path $PSScriptRoot 'OmniphonyEndpointCtl.vcxproj'
    if (-not (Test-Path -LiteralPath $helperSource)) { throw "Native endpoint helper source missing: $helperSource" }
    if (-not (Test-Path -LiteralPath $helperProject)) { throw "Native endpoint helper project missing: $helperProject" }
    Write-Host 'Omniphony for Windows installer control plane validated without runtime C# compilation.'
    exit 0
}

Require-Administrator
$driverRoot = Join-Path $AppRoot 'driver'
$driverScript = Join-Path $driverRoot 'SpatialEndpoint.ps1'
$certificate = Join-Path $driverRoot 'SpatialEndpoint-Development.cer'
$exe = Join-Path $AppRoot 'Omniphony.exe'

try {
    if ($Action -eq 'Install') {
        Write-InstallLog 'Install requested.'
        Stop-OldHosts
        Remove-OmniphonyRunEntry

        if (-not (Test-Path $exe)) { throw "Omniphony runtime missing: $exe" }
        if (-not (Test-Path $driverScript)) { throw "Endpoint installer missing: $driverScript" }
        if (-not (Test-Path $EndpointCtlPath)) { throw "Endpoint control helper missing: $EndpointCtlPath" }

        $defaultResult = Invoke-EndpointCtlRaw @('get-default')
        if ($defaultResult.ExitCode -eq 0) {
            Write-InstallLog "Pre-install default render endpoint: $($defaultResult.Lines -join ' | ')"
        } else {
            Write-InstallLog "Pre-install default render endpoint probe warning: $($defaultResult.Lines -join ' | ')"
        }

        # Private-development priority: prefer Valve's already Microsoft-trusted
        # Steam Streaming Speakers when the local Steam installation provides it.
        # This keeps stock Windows 11/Secure Boot intact. The Valve driver is only
        # transport; it is neither redistributed nor modified nor owned by Omniphony.
        $steamAvailable = (Get-EndpointIdByFriendlyName @('Steam Streaming Speakers')) -or
                          ($SteamDriverInf -and (Test-Path -LiteralPath $SteamDriverInf))

        if ($steamAvailable) {
            Install-SteamTransport
        }
        else {
            try {
                Install-DevelopmentOmniphonyEndpoint $driverScript $certificate
            }
            catch {
                try { & $driverScript -Action Remove } catch { Write-InstallLog "Development endpoint cleanup warning: $($_.Exception.Message)" }
                try { Remove-DevelopmentCertificate $certificate } catch { Write-InstallLog "Development certificate cleanup warning: $($_.Exception.Message)" }

                $secureBoot = $false
                try { $secureBoot = Confirm-SecureBootUEFI -ErrorAction Stop } catch { }
                $detail = if ($secureBoot) {
                    'Windows 11 blocked the development/test-signed Omniphony endpoint and no signed Steam Streaming Speakers transport was available locally. The permanent fix is Microsoft production signing of the Omniphony driver.'
                } else {
                    'Windows rejected the development Omniphony endpoint and no signed Steam Streaming Speakers transport was available locally.'
                }
                Write-InstallLog "$detail Driver error: $($_.Exception.Message)"
                throw $detail
            }
        }

        # The runtime supervisor is the canonical owner of the per-user
        # "Start with Windows" preference. It registers its own Run value with
        # reg.exe on first launch and treats policy denial as non-fatal. The
        # elevated installer must not make a redundant HKCU write capable of
        # rolling back an otherwise working audio installation.
        Remove-LegacyRunEntries
        Write-InstallLog 'Runtime supervisor owns per-user autostart; installer skipped HKCU Run registration.'
        Write-InstallLog "Physical output preference: $PhysicalOutput"
        Write-InstallLog 'Install control plane completed successfully.'
        exit 0
    }

    Write-InstallLog 'Uninstall requested.'
    Stop-OldHosts
    try {
        Set-DefaultEndpointByName @($PhysicalOutput, 'FiiO') | Out-Null
    }
    catch {
        Write-InstallLog "Physical default restore warning: $($_.Exception.Message)"
    }

    # Remove only Omniphony-owned development machinery. A Valve Steam endpoint
    # used as a bootstrap transport is intentionally left untouched.
    if (Test-Path $driverScript) {
        try { & $driverScript -Action Remove } catch { Write-InstallLog "Endpoint removal warning: $($_.Exception.Message)" }
    }
    Remove-OmniphonyRunEntry
    Remove-DevelopmentCertificate $certificate
    Remove-TransportState
    Write-InstallLog 'Uninstall control-plane cleanup completed.'
    exit 0
}
catch {
    $failure = $_
    if ($Action -eq 'Install') {
        Write-InstallLog 'Install failed; rolling back Omniphony-owned machine state.'
        Stop-OldHosts
        Remove-OmniphonyRunEntry
        if (Test-Path $driverScript) {
            try { & $driverScript -Action Remove } catch { Write-InstallLog "Rollback endpoint removal warning: $($_.Exception.Message)" }
        }
        try { Remove-DevelopmentCertificate $certificate } catch { Write-InstallLog "Rollback certificate removal warning: $($_.Exception.Message)" }
        Remove-TransportState
        try {
            Set-DefaultEndpointByName @($PhysicalOutput, 'FiiO') | Out-Null
        }
        catch {
            Write-InstallLog "Rollback physical default restore warning: $($_.Exception.Message)"
        }
    }
    Write-InstallLog "FATAL: $($failure.Exception.Message)"
    Write-Error $failure
    exit 1603
}