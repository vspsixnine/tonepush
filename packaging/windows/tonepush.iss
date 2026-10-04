; Windows installer built from a release build with Inno Setup 6.3 or newer:
;
;   iscc /DVersion=0.9.0 /DArch=x86_64 /DBinaries=...\target\x86_64-pc-windows-msvc\release ^
;        /DOutputDir=dist packaging\windows\tonepush.iss
;
; Arch matches the Rust target: x86_64 or aarch64. Binaries is the folder
; holding tonepush-gui.exe (the editor) and tonepush.exe (the command line).
; Installation uses the current user's Programs folder and does not need
; administrator rights. Updates close a running copy before replacing it.

#ifndef Version
  #error Version must be defined on the ISCC command line
#endif
#ifndef Arch
  #error Arch must be defined on the ISCC command line (x86_64 or aarch64)
#endif
#ifndef NumericVersion
  #define NumericVersion Version
#endif
#ifndef Binaries
  #error Binaries must be defined on the ISCC command line
#endif
#ifndef OutputDir
  #error OutputDir must be defined on the ISCC command line
#endif
#if Arch == "aarch64"
  #define InnoArch "arm64"
#else
  #define InnoArch "x64compatible"
#endif

#define AppName "TonePush"
#define AppExeName "tonepush-gui.exe"

[Setup]
; Never change: this is how Windows tells an update from a new program.
AppId={{E7F9B874-71BE-4DE7-8323-BB09AFBD702F}
AppName={#AppName}
AppVersion={#Version}
AppVerName={#AppName} {#Version}
AppPublisher=Carmine Paolino
AppCopyright=© 2026 Carmine Paolino
AppPublisherURL=https://tonepush.rocks
AppSupportURL=https://github.com/crmne/tonepush/issues
AppUpdatesURL=https://github.com/crmne/tonepush/releases
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed={#InnoArch}
ArchitecturesInstallIn64BitMode={#InnoArch}
MinVersion=10.0
LicenseFile=..\..\LICENSE
OutputDir={#OutputDir}
OutputBaseFilename=tonepush-v{#Version}-{#Arch}-pc-windows-msvc-setup
SetupIconFile=tonepush.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\{#AppExeName}
VersionInfoVersion={#NumericVersion}.0

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#Binaries}\tonepush-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Binaries}\tonepush.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
; Tells TonePush's updater that this copy came from the installer, so an
; update downloads and runs the next installer instead of an archive.
Source: "tonepush-installer.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
