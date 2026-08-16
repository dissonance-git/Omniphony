#ifndef PayloadDir
  #error PayloadDir must be supplied by the build workflow
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

#define MyAppName "Omniphony for Windows"
#define MyAppVersion "0.0.1-dev"
#define MyAppPublisher "Omniphony downstream fork"
#define MyAppExeName "Omniphony.exe"

[Setup]
AppId={{6A6873B9-1199-4D6B-AC3E-9415E5BC6BB1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\Omniphony
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableWelcomePage=yes
DisableDirPage=yes
DisableReadyPage=yes
DisableFinishedPage=yes
ShowLanguageDialog=no
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
SetupLogging=yes
OutputDir={#OutputDir}
OutputBaseFilename=OmniphonySetup
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=no

[Files]
Source: "{#PayloadDir}\app\Omniphony.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\driver\*"; DestDir: "{app}\driver"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PayloadDir}\support\Install-OmniphonyForWindows.ps1"; DestDir: "{app}\support"; Flags: ignoreversion
Source: "{#PayloadDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\PRODUCT-CONTEXT.md"; DestDir: "{app}"; Flags: ignoreversion

[Run]
Filename: "{app}\Omniphony.exe"; Description: "Start Omniphony"; Flags: nowait runasoriginaluser skipifsilent

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\support\Install-OmniphonyForWindows.ps1"" -Action Uninstall -AppRoot ""{app}"" -PhysicalOutput ""Dan Clark Noire X"""; Flags: runhidden waituntilterminated; RunOnceId: "OmniphonyForWindowsCleanup"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
  PowerShell: String;
  Params: String;
begin
  if CurStep = ssPostInstall then
  begin
    PowerShell := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
    Params := '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' +
      ExpandConstant('{app}\support\Install-OmniphonyForWindows.ps1') +
      '" -Action Install -AppRoot "' + ExpandConstant('{app}') +
      '" -PhysicalOutput "Dan Clark Noire X"';

    if (not Exec(PowerShell, Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)) or
       (ResultCode <> 0) then
    begin
      RaiseException(
        'Omniphony installation could not finish. The development audio driver may have been blocked by Windows 11 driver-signing policy. ' +
        'See C:\ProgramData\Omniphony\installer.log for the exact boundary, then run this same installer again after resolving it.'
      );
    end;
  end;
end;
