; ========================================================
; FocusGrow Professional Windows Setup Script (Inno Setup 6)
; Native Rust & DirectX High-Performance Architecture
; ========================================================

#define MyAppName "FocusGrow"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Genesfi"
#define MyAppURL "https://github.com/Genesfi/focusgrow"
#define MyAppExeName "FocusGrow.exe"

[Setup]
; Unique AppId generated for FocusGrow
AppId={{D8A1C52E-5B39-44F2-8729-9F89B2B612C9}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} v{#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
; Application icon
SetupIconFile=app.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
; Modern styling and high compression
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
OutputDir=dist
OutputBaseFilename=FocusGrow_Setup_v{#MyAppVersion}
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesInstallIn64BitMode=x64compatible
; Information before installation (Read This Before Install)
InfoBeforeFile=INSTALLER_INFO.txt

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "startupicon"; Description: "Run FocusGrow automatically when Windows starts"; GroupDescription: "Startup Options:"; Flags: unchecked

[Files]
; Main executable (Pure native Rust & DirectX/Direct2D)
Source: "bin\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Application icon & documentation
Source: "app.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "INSTALLER_INFO.txt"; DestDir: "{app}"; Flags: ignoreversion
; FocusGrow Browser Extension (Restricted Sites & Cooldown Guard)
Source: "bin\extension-focusgrow\*"; DestDir: "{app}\extension-focusgrow"; Flags: ignoreversion recursesubdirs createallsubdirs
; YouTube Music Media Browser Extension (Chrome/Edge/Brave companion)
Source: "bin\extension-ytmpx\*"; DestDir: "{app}\extension-ytmpx"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
Name: "{autoprograms}\{#MyAppName}\FocusGrow Extension Folder"; Filename: "{win}\explorer.exe"; Parameters: """{app}\extension-focusgrow"""
Name: "{autoprograms}\{#MyAppName}\YT Music Extension Folder"; Filename: "{win}\explorer.exe"; Parameters: """{app}\extension-ytmpx"""
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\app.ico"
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startupicon; IconFilename: "{app}\app.ico"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
Filename: "{win}\explorer.exe"; Parameters: """{app}\extension-focusgrow"""; Description: "Open FocusGrow Site Guard Extension folder (load in chrome://extensions)"; Flags: nowait postinstall skipifsilent unchecked
Filename: "{win}\explorer.exe"; Parameters: """{app}\extension-ytmpx"""; Description: "Open YouTube Music Extension folder (load in chrome://extensions)"; Flags: nowait postinstall skipifsilent unchecked

[UninstallDelete]
; Clean up runtime cache, local session databases, and temporary files generated after install
Type: filesandordirs; Name: "{app}\data"
Type: filesandordirs; Name: "{app}\extension-focusgrow"
Type: filesandordirs; Name: "{app}\extension-ytmpx"
Type: filesandordirs; Name: "{app}\*"
Type: dirifempty; Name: "{app}"

[Code]
// Helper to terminate active instances so files are not locked during install/uninstall
procedure KillProcess(ProcessName: String);
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM ' + ProcessName + ' /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function InitializeSetup(): Boolean;
begin
  KillProcess('FocusGrow.exe');
  KillProcess('ytmpx_ui.exe');
  KillProcess('ytmpx-core.exe');
  KillProcess('node.exe');
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    KillProcess('FocusGrow.exe');
    KillProcess('ytmpx_ui.exe');
    KillProcess('ytmpx-core.exe');
    KillProcess('node.exe');
    Sleep(500);
  end;
end;

