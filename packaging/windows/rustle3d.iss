; Inno Setup script for the Windows installer, built by scripts/release.sh:
;   ISCC /DAppVersion=<version> /DSourceDir=<staged app folder>
;        /DOutputDir=<dist> /DOutputName=<file name without .exe> rustle3d.iss
; Installs for the current user (no administrator rights) unless the user
; chooses all users, adds a Start menu entry (and a desktop one on request)
; and an uninstaller. Unsigned for now, like every package.

#ifndef AppVersion
  #error AppVersion is not defined (pass /DAppVersion=<version>)
#endif
#ifndef SourceDir
  #error SourceDir is not defined (pass /DSourceDir=<staged app folder>)
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef OutputName
  #define OutputName "Rustle3D-" + AppVersion + "-windows-x64-setup"
#endif

[Setup]
; The AppId identifies the installation for upgrades and uninstall: never change it.
AppId={{5E0C6E2B-7D3A-4F61-9B5C-2A8E4C1F0D73}
AppName=Rustle3D
AppVersion={#AppVersion}
AppVerName=Rustle3D {#AppVersion}
AppPublisher=The Rustle3D authors
AppPublisherURL=https://app.rustle3d.com
AppSupportURL=https://app.rustle3d.com
AppUpdatesURL=https://app.rustle3d.com/en/download/
AppCopyright=Copyright (C) 2026 the Rustle3D authors. GNU GPL v3 or later.
DefaultDirName={autopf}\Rustle3D
DefaultGroupName=Rustle3D
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; x64 build; Windows on Arm runs it through its x64 emulation.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile={#SourceDir}\LICENSE
SetupIconFile={#SourcePath}..\..\app\windows\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\rustle3d.exe
UninstallDisplayName=Rustle3D
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Rustle3D"; Filename: "{app}\rustle3d.exe"
Name: "{autodesktop}\Rustle3D"; Filename: "{app}\rustle3d.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\rustle3d.exe"; Description: "{cm:LaunchProgram,Rustle3D}"; Flags: nowait postinstall skipifsilent
