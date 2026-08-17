#ifndef PayloadDir
  #error PayloadDir must be supplied by the build workflow
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

#define MyAppName "Omniphony for Windows"
#define MyAppVersion "0.0.2-dev"
#define MyAppPublisher "Omniphony downstream fork"

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

; Migration cleanup from the abandoned virtual-device / loopback host product.
; These run before the new files are copied, after PrepareToInstall has killed
; any legacy Omniphony.exe instance.
[InstallDelete]
Type: files; Name: "{app}\Omniphony.exe"
Type: files; Name: "{app}\PRODUCT-CONTEXT.md"
Type: filesandordirs; Name: "{app}\driver"
Type: filesandordirs; Name: "{app}\EndpointAPO"
Type: filesandordirs; Name: "{app}\support"

[Files]
; Runtime files are staged only for the duration of setup. The APO installer
; stops AudioSrv and copies them into {app}\APO, which also makes future upgrades
; safe when AudioDG has the old DLL loaded.
Source: "{#PayloadDir}\runtime\*"; DestDir: "{tmp}\OmniphonyAPOPayload"; Flags: ignoreversion recursesubdirs createallsubdirs deleteafterinstall
Source: "{#PayloadDir}\support\*"; DestDir: "{app}\support"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PayloadDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\support\Uninstall-OmniphonyAPO.ps1"" -AppRoot ""{app}"" -PhysicalOutput ""Dan Clark Noire X"""; Flags: runhidden waituntilterminated; RunOnceId: "OmniphonyApoCleanup"

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
  TaskKill: String;
begin
  { Explicit migration invariant: the old tray/loopback host must be gone before
    stale files are deleted or the physical endpoint is reconfigured. taskkill
    returns a nonzero code when no matching process exists, which is harmless. }
  TaskKill := ExpandConstant('{sys}\taskkill.exe');
  Exec(TaskKill, '/F /T /IM Omniphony.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);

  { Retire both historical autostart names so the obsolete host cannot return on
    the next login after an APO-native upgrade. }
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
    Params := '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' +
      ExpandConstant('{app}\support\Install-OmniphonyAPO.ps1') +
      '" -PackageRoot "' + ExpandConstant('{tmp}\OmniphonyAPOPayload') +
      '" -AppRoot "' + ExpandConstant('{app}') +
      '" -PhysicalOutput "Dan Clark Noire X"';

    if (not Exec(PowerShell, Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)) or
       (ResultCode <> 0) then
    begin
      RaiseException(
        'Omniphony APO installation could not finish safely. The installer performs automatic endpoint rollback on APO/WASAPI failure.'
      );
    end;
  end;
end;
