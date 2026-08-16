param(
    [ValidateSet('Install', 'Uninstall', 'Validate')]
    [string]$Action = 'Install',

    [Parameter(Mandatory = $true)]
    [string]$AppRoot,

    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'

$installer = Join-Path $PSScriptRoot 'Install-OmniphonyForWindows.ps1'
if (-not (Test-Path -LiteralPath $installer)) {
    throw "Omniphony installer control plane is missing: $installer"
}

# Windows 11 still ships Windows PowerShell 5.1. Its Add-Type path uses the
# legacy CodeDOM compiler, which does not understand C# 7 pattern variables
# such as `client is IPolicyConfig modern`. Keep the canonical installer
# readable/newer while compiling its embedded COM shim through a strictly
# C#-5-compatible temporary copy on the stock Windows runtime.
$text = Get-Content -LiteralPath $installer -Raw
$replacements = @(
    @(
        'if (client is IPolicyConfig modern)',
        "IPolicyConfig modern = client as IPolicyConfig;`r`n                    if (modern != null)"
    ),
    @(
        'if (client is IPolicyConfigVista vista)',
        "IPolicyConfigVista vista = client as IPolicyConfigVista;`r`n                    if (vista != null)"
    )
)

foreach ($replacement in $replacements) {
    $old = [string]$replacement[0]
    $new = [string]$replacement[1]
    if (-not $text.Contains($old)) {
        throw "Windows PowerShell compatibility source drift: expected token not found: $old"
    }
    $text = $text.Replace($old, $new)
}

$tempRoot = Join-Path $env:TEMP 'Omniphony'
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
$patched = Join-Path $tempRoot ("Install-OmniphonyForWindows-{0}.ps1" -f [Guid]::NewGuid().ToString('N'))

try {
    # UTF-8 with BOM is deliberate for Windows PowerShell 5.1.
    $utf8Bom = New-Object System.Text.UTF8Encoding($true)
    [System.IO.File]::WriteAllText($patched, $text, $utf8Bom)

    $windowsPowerShell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
    if (-not (Test-Path -LiteralPath $windowsPowerShell)) {
        throw "Windows PowerShell 5.1 was not found: $windowsPowerShell"
    }

    $invokeArgs = @(
        '-NoProfile',
        '-ExecutionPolicy', 'Bypass',
        '-File', $patched,
        '-Action', $Action,
        '-AppRoot', $AppRoot,
        '-PhysicalOutput', $PhysicalOutput
    )

    & $windowsPowerShell @invokeArgs
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 1 }
    exit $code
}
finally {
    Remove-Item -LiteralPath $patched -Force -ErrorAction SilentlyContinue
}
