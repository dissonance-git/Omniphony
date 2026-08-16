param(
    [ValidateSet('Install', 'Uninstall', 'Validate')]
    [string]$Action = 'Install',

    [Parameter(Mandatory = $true)]
    [string]$AppRoot,

    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$LogPath = Join-Path $env:ProgramData 'Omniphony\installer.log'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$EndpointHardwareId = 'ROOT\SpatialAudioEndpoint' # stable private bootstrap ID
$CurrentEndpointServiceName = 'VirtualAudioDriver'

function Write-InstallLog([string]$Message) {
    $dir = Split-Path -Parent $LogPath
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    Add-Content -LiteralPath $LogPath -Value "[$(Get-Date -Format o)] $Message" -Encoding utf8
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
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
        Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    }

    $legacyServices = @(Get-CimInstance Win32_Service -ErrorAction SilentlyContinue | Where-Object { Test-LegacyOmniphonyService $_ })
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

function Get-EndpointIdByFriendlyName([string[]]$Needles) {
    $root = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render'
    if (-not (Test-Path $root)) { return $null }

    foreach ($endpoint in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
        $propertiesPath = Join-Path $endpoint.PSPath 'Properties'
        if (-not (Test-Path $propertiesPath)) { continue }
        $properties = Get-ItemProperty -LiteralPath $propertiesPath -ErrorAction SilentlyContinue
        if (-not $properties) { continue }

        $strings = @($properties.PSObject.Properties | ForEach-Object {
            if ($_.Value -is [string]) { $_.Value }
        })
        foreach ($needle in $Needles) {
            if ($strings | Where-Object { $_ -like "*$needle*" }) {
                return $endpoint.PSChildName
            }
        }
    }
    return $null
}

function Ensure-PolicyConfigInterop {
    if ('Omniphony.WindowsAudio.PolicyConfig' -as [type]) { return }

    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Omniphony.WindowsAudio
{
    public enum ERole
    {
        Console = 0,
        Multimedia = 1,
        Communications = 2
    }

    [ComImport]
    [Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
    public class PolicyConfigClientComObject { }

    [ComImport]
    [Guid("294935CE-F637-4E7C-A41B-AB255460B862")]
    public class PolicyConfigVistaClientComObject { }

    [ComImport]
    [Guid("f8679f50-850a-455c-9d37-1cfe6b8a1b8e")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IPolicyConfig
    {
        [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, out IntPtr format);
        [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int isDefault, out IntPtr format);
        [PreserveSig] int ResetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId);
        [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr endpointFormat, IntPtr mixFormat);
        [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int isDefault, out long defaultPeriod, out long minimumPeriod);
        [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string deviceId, ref long period);
        [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string deviceId, out IntPtr mode);
        [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr mode);
        [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr key, out IntPtr value);
        [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr key, IntPtr value);
        [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string deviceId, ERole role);
        [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int visible);
    }

    [ComImport]
    [Guid("568b9108-44bf-40b4-9006-86afe5b5a620")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IPolicyConfigVista
    {
        [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, out IntPtr format);
        [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int isDefault, out IntPtr format);
        [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr endpointFormat, IntPtr mixFormat);
        [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int isDefault, out long defaultPeriod, out long minimumPeriod);
        [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string deviceId, ref long period);
        [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string deviceId, out IntPtr mode);
        [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr mode);
        [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr key, out IntPtr value);
        [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string deviceId, IntPtr key, IntPtr value);
        [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string deviceId, ERole role);
        [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int visible);
    }

    public static class PolicyConfig
    {
        public static void SetDefault(string deviceId)
        {
            Exception last = null;
            foreach (Func<object> create in Candidates())
            {
                object client = null;
                try
                {
                    client = create();
                    if (client is IPolicyConfig modern)
                    {
                        SetAll(deviceId, modern.SetDefaultEndpoint);
                        Release(client);
                        return;
                    }
                    if (client is IPolicyConfigVista vista)
                    {
                        SetAll(deviceId, vista.SetDefaultEndpoint);
                        Release(client);
                        return;
                    }
                }
                catch (Exception ex)
                {
                    last = ex;
                }
                finally
                {
                    Release(client);
                }
            }
            throw new InvalidOperationException("No usable Windows PolicyConfig interface was available.", last);
        }

        private static IEnumerable<Func<object>> Candidates()
        {
            yield return () => new PolicyConfigClientComObject();
            yield return () => new PolicyConfigVistaClientComObject();
        }

        private static void SetAll(string deviceId, Func<string, ERole, int> setter)
        {
            foreach (ERole role in new[] { ERole.Console, ERole.Multimedia, ERole.Communications })
            {
                int hr = setter(deviceId, role);
                if (hr < 0) Marshal.ThrowExceptionForHR(hr);
            }
        }

        private static void Release(object obj)
        {
            if (obj != null && Marshal.IsComObject(obj))
                Marshal.FinalReleaseComObject(obj);
        }
    }
}
'@
}

function Set-DefaultEndpointByName([string[]]$Needles) {
    Ensure-PolicyConfigInterop
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        $id = Get-EndpointIdByFriendlyName $Needles
        if ($id) {
            [Omniphony.WindowsAudio.PolicyConfig]::SetDefault($id)
            Write-InstallLog "Default render endpoint set to $($Needles -join ' / ') [$id]"
            return $id
        }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Windows did not expose an active render endpoint matching: $($Needles -join ', ')"
}

function Import-DevelopmentCertificate([string]$CertificatePath) {
    if (-not (Test-Path $CertificatePath)) { throw "Development certificate missing: $CertificatePath" }
    $cert = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
    Import-Certificate -FilePath $CertificatePath -CertStoreLocation 'Cert:\LocalMachine\TrustedPublisher' | Out-Null
    Write-InstallLog "Imported development driver certificate $($cert.Thumbprint)"
    return $cert.Thumbprint
}

function Remove-DevelopmentCertificate([string]$CertificatePath) {
    if (-not (Test-Path $CertificatePath)) { return }
    $cert = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
    foreach ($store in @('Root', 'TrustedPublisher')) {
        $path = "Cert:\LocalMachine\$store\$($cert.Thumbprint)"
        if (Test-Path $path) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue }
    }
    Write-InstallLog "Removed development driver certificate $($cert.Thumbprint)"
}

if ($Action -eq 'Validate') {
    Ensure-PolicyConfigInterop
    Write-Host 'Omniphony for Windows installer control plane compiled successfully.'
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

        Import-DevelopmentCertificate $certificate | Out-Null

        try {
            & $driverScript -Action Install
        }
        catch {
            $secureBoot = $false
            try { $secureBoot = Confirm-SecureBootUEFI -ErrorAction Stop } catch { }
            $detail = if ($secureBoot) {
                'Windows 11 blocked the development/test-signed Omniphony audio driver while Secure Boot enforcement is active. This private build cannot legitimately bypass that kernel policy. Disable Driver Signature Enforcement for the current boot, then run the same installer again.'
            } else {
                'Windows rejected the development Omniphony audio driver. Run the same installer again after booting Windows with Driver Signature Enforcement disabled for this development boot.'
            }
            Write-InstallLog "$detail Driver error: $($_.Exception.Message)"
            throw $detail
        }

        Set-DefaultEndpointByName @('Omniphony', 'Spatial') | Out-Null

        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $RunKey) | Out-Null
        New-ItemProperty -LiteralPath $RunKey -Name 'Omniphony' -PropertyType String -Value ('"' + $exe + '"') -Force | Out-Null
        Remove-LegacyRunEntries
        Write-InstallLog "Autostart configured: $exe"
        Write-InstallLog "Physical output preference: $PhysicalOutput"
        Write-InstallLog 'Install control plane completed successfully.'
        exit 0
    }

    Write-InstallLog 'Uninstall requested.'
    Stop-OldHosts
    try { Set-DefaultEndpointByName @($PhysicalOutput, 'FiiO') | Out-Null } catch { Write-InstallLog "Physical default restore warning: $($_.Exception.Message)" }
    if (Test-Path $driverScript) {
        try { & $driverScript -Action Remove } catch { Write-InstallLog "Endpoint removal warning: $($_.Exception.Message)" }
    }
    Remove-OmniphonyRunEntry
    Remove-DevelopmentCertificate $certificate
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
        try { Set-DefaultEndpointByName @($PhysicalOutput, 'FiiO') | Out-Null } catch { Write-InstallLog "Rollback physical default restore warning: $($_.Exception.Message)" }
    }
    Write-InstallLog "FATAL: $($failure.Exception.Message)"
    Write-Error $failure
    exit 1603
}
