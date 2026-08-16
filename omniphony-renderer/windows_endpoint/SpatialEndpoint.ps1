param(
    [ValidateSet('Install', 'Remove', 'Status', 'Validate')]
    [string]$Action = 'Install'
)

$ErrorActionPreference = 'Stop'
$HardwareId = 'ROOT\SpatialAudioEndpoint'

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Require-Administrator {
    if (-not (Test-Administrator)) {
        throw 'Spatial endpoint installation/removal must be run from an Administrator PowerShell or elevated launcher.'
    }
}

function Invoke-PnpUtil {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [switch]$AllowFailure
    )

    & "$env:WINDIR\System32\pnputil.exe" @Arguments
    $code = $LASTEXITCODE
    if ($code -ne 0 -and -not $AllowFailure) {
        throw "PnPUtil failed with exit code ${code}: $($Arguments -join ' ')"
    }
    return $code
}

function Ensure-SetupApiBridge {
    if ('Spatial.Endpoint.SetupApi' -as [type]) {
        return
    }

    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

namespace Spatial.Endpoint
{
    public static class SetupApi
    {
        private const uint DICD_GENERATE_ID = 0x00000001;
        private const uint SPDRP_HARDWAREID = 0x00000001;
        private const uint DIF_REGISTERDEVICE = 0x00000019;
        private static readonly IntPtr INVALID_HANDLE_VALUE = new IntPtr(-1);

        [StructLayout(LayoutKind.Sequential)]
        private struct SP_DEVINFO_DATA
        {
            public uint cbSize;
            public Guid ClassGuid;
            public uint DevInst;
            public UIntPtr Reserved;
        }

        [DllImport("setupapi.dll", EntryPoint = "SetupDiGetINFClassW", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool SetupDiGetINFClass(
            string InfName,
            out Guid ClassGuid,
            StringBuilder ClassName,
            uint ClassNameSize,
            out uint RequiredSize);

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern IntPtr SetupDiCreateDeviceInfoList(
            ref Guid ClassGuid,
            IntPtr hwndParent);

        [DllImport("setupapi.dll", EntryPoint = "SetupDiCreateDeviceInfoW", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool SetupDiCreateDeviceInfo(
            IntPtr DeviceInfoSet,
            string DeviceName,
            ref Guid ClassGuid,
            string DeviceDescription,
            IntPtr hwndParent,
            uint CreationFlags,
            ref SP_DEVINFO_DATA DeviceInfoData);

        [DllImport("setupapi.dll", EntryPoint = "SetupDiSetDeviceRegistryPropertyW", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool SetupDiSetDeviceRegistryProperty(
            IntPtr DeviceInfoSet,
            ref SP_DEVINFO_DATA DeviceInfoData,
            uint Property,
            byte[] PropertyBuffer,
            uint PropertyBufferSize);

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiCallClassInstaller(
            uint InstallFunction,
            IntPtr DeviceInfoSet,
            ref SP_DEVINFO_DATA DeviceInfoData);

        [DllImport("setupapi.dll", SetLastError = true)]
        private static extern bool SetupDiDestroyDeviceInfoList(IntPtr DeviceInfoSet);

        private static void Win32(bool ok, string operation)
        {
            if (!ok)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), operation);
            }
        }

        public static void CreateRootDevice(string infPath, string hardwareId)
        {
            Guid classGuid;
            uint required;
            var className = new StringBuilder(256);
            Win32(
                SetupDiGetINFClass(infPath, out classGuid, className, (uint)className.Capacity, out required),
                "SetupDiGetINFClass failed");

            IntPtr set = SetupDiCreateDeviceInfoList(ref classGuid, IntPtr.Zero);
            if (set == INVALID_HANDLE_VALUE)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "SetupDiCreateDeviceInfoList failed");
            }

            try
            {
                var data = new SP_DEVINFO_DATA();
                data.cbSize = (uint)Marshal.SizeOf(typeof(SP_DEVINFO_DATA));

                Win32(
                    SetupDiCreateDeviceInfo(
                        set,
                        className.ToString(),
                        ref classGuid,
                        null,
                        IntPtr.Zero,
                        DICD_GENERATE_ID,
                        ref data),
                    "SetupDiCreateDeviceInfo failed");

                // SPDRP_HARDWAREID is REG_MULTI_SZ. One hardware ID therefore
                // needs its terminator plus the list terminator.
                byte[] ids = Encoding.Unicode.GetBytes(hardwareId + "\0\0");
                Win32(
                    SetupDiSetDeviceRegistryProperty(
                        set,
                        ref data,
                        SPDRP_HARDWAREID,
                        ids,
                        (uint)ids.Length),
                    "SetupDiSetDeviceRegistryProperty(SPDRP_HARDWAREID) failed");

                Win32(
                    SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set, ref data),
                    "SetupDiCallClassInstaller(DIF_REGISTERDEVICE) failed");
            }
            finally
            {
                SetupDiDestroyDeviceInfoList(set);
            }
        }
    }
}
'@
}

function Get-EndpointInf {
    $candidates = @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.inf' -File)
    if ($candidates.Count -ne 1) {
        throw "Expected exactly one Spatial endpoint INF beside this script; found $($candidates.Count)."
    }
    return $candidates[0].FullName
}

switch ($Action) {
    'Validate' {
        # CI/dev check: compile the embedded SetupAPI bridge without changing
        # device state. This catches P/Invoke/C# syntax drift before packaging.
        Ensure-SetupApiBridge
        Write-Host 'Spatial endpoint installer bridge compiled successfully.'
        break
    }

    'Status' {
        Invoke-PnpUtil -Arguments @('/enum-devices', '/deviceid', $HardwareId) -AllowFailure | Out-Null
        break
    }

    'Remove' {
        Require-Administrator
        # Windows 11 supports device removal by hardware/device ID. Removing the
        # devnode is sufficient to make the private endpoint disappear. A staged
        # development driver package may remain in DriverStore for later reuse.
        Invoke-PnpUtil -Arguments @('/remove-device', '/deviceid', $HardwareId) -AllowFailure | Out-Null
        Write-Host 'Spatial endpoint removal requested.'
        break
    }

    'Install' {
        Require-Administrator
        $inf = Get-EndpointInf

        # Refresh in place by removing only our uniquely named root device first.
        # Do not modify Secure Boot, BitLocker, boot configuration, or test-signing
        # policy here. Driver trust is an explicit machine/user evidence boundary.
        Invoke-PnpUtil -Arguments @('/remove-device', '/deviceid', $HardwareId) -AllowFailure | Out-Null

        Ensure-SetupApiBridge
        try {
            [Spatial.Endpoint.SetupApi]::CreateRootDevice($inf, $HardwareId)
            Invoke-PnpUtil -Arguments @('/add-driver', $inf, '/install') | Out-Null
        }
        catch {
            # Avoid leaving a driverless root devnode behind when package trust or
            # installation fails.
            Invoke-PnpUtil -Arguments @('/remove-device', '/deviceid', $HardwareId) -AllowFailure | Out-Null
            throw
        }

        Write-Host 'Spatial endpoint installed. Windows should now expose one output device named Spatial.'
        Write-Host 'This installer intentionally did not change Secure Boot, BitLocker, or Windows test-signing state.'
        break
    }
}
