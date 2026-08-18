#ifndef PayloadDir
  #error PayloadDir must be supplied by the build workflow
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

#define MyAppName "Omniphony for Windows"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Omniphony"

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
CloseApplications=yes
RestartApplications=no

; Migration cleanup from abandoned virtual-device / loopback-host builds.
[InstallDelete]
Type: files; Name: "{app}\Omniphony.exe"
Type: files; Name: "{app}\PRODUCT-CONTEXT.md"
Type: filesandordirs; Name: "{app}\driver"
Type: filesandordirs; Name: "{app}\EndpointAPO"
Type: filesandordirs; Name: "{app}\support"

; The renderer/APO is administrator-owned. Only small preference/log state is
; user-writable so the tray can change listener options without UAC.
[Dirs]
Name: "{commonappdata}\Omniphony"; Permissions: users-modify

[Files]
; Runtime files are staged only for setup. Install-OmniphonyAPO.ps1 places the
; two AudioDG-loaded DLLs under Program Files and attaches Current to the current
; default render endpoint.
Source: "{#PayloadDir}\runtime\*"; DestDir: "{tmp}\OmniphonyAPOPayload"; Flags: ignoreversion recursesubdirs createallsubdirs deleteafterinstall
Source: "{#PayloadDir}\support\*"; DestDir: "{app}\support"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "endpoint_apo\OmniphonyTray.ps1"; DestDir: "{app}\support"; Flags: ignoreversion
Source: "{#PayloadDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

; Omniphony is headless. The tray icon is the only normal UI surface.
[Icons]
Name: "{userstartup}\Omniphony"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\support\OmniphonyTray.ps1"""; WorkingDir: "{app}\support"

[Run]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\support\OmniphonyTray.ps1"""; WorkingDir: "{app}\support"; Flags: nowait runhidden runasoriginaluser

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command ""New-Item -ItemType Directory -Force -Path '{commonappdata}\Omniphony' | Out-Null; Set-Content -LiteralPath '{commonappdata}\Omniphony\tray.stop' -Value stop -Encoding ASCII"""; Flags: runhidden waituntilterminated; RunOnceId: "OmniphonyTrayStop"
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\support\Uninstall-OmniphonyAPO.ps1"" -AppRoot ""{app}"""; Flags: runhidden waituntilterminated; RunOnceId: "OmniphonyApoCleanup"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
  TaskKill: String;
  TrayStop: String;
begin
  { Ask an existing tray instance to leave before support files are replaced. }
  ForceDirectories(ExpandConstant('{commonappdata}\Omniphony'));
  TrayStop := ExpandConstant('{commonappdata}\Omniphony\tray.stop');
  SaveStringToFile(TrayStop, 'stop', False);

  { Retire the old standalone/loopback host before upgrading. }
  TaskKill := ExpandConstant('{sys}\taskkill.exe');
  Exec(TaskKill, '/F /T /IM Omniphony.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

  RegDeleteValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Omniphony');
  RegDeleteValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Spatial');

  NeedsRestart := False;
  Result := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
  PowerShell: String;
  Params: String;
begin
  if CurStep = ssPostInstall then
  begin
    PowerShell := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');

    { Normal Omniphony 0.1 deployment is an unsigned user-mode endpoint APO.
      This intentionally uses the unprotected AudioDG compatibility mode, then
      runs Current headlessly on the selected Windows endpoint. }
    Params := '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' +
      ExpandConstant('{app}\support\Install-OmniphonyAPO.ps1') +
      '" -PackageRoot "' + ExpandConstant('{tmp}\OmniphonyAPOPayload') +
      '" -AppRoot "' + ExpandConstant('{app}') + '" -AllowUnprotectedAudioDG';

    if (not Exec(PowerShell, Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)) or
       (ResultCode <> 0) then
    begin
      RaiseException(
        'Omniphony could not attach to the current Windows output. The previous endpoint state was restored automatically. Diagnostic log: C:\ProgramData\Omniphony\install-last.log'
      );
    end;
  end;
end;
