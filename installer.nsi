!include "MUI2.nsh"

Name "PDFbull"
OutFile "PDFbull-Setup.exe"
InstallDir "$PROGRAMFILES64\PDFbull"
RequestExecutionLevel admin

!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

Section "Install"
    SetOutPath "$INSTDIR"
    
    File "release_dist\pdfbull.exe"
    File "release_dist\pdfium.dll"
    File "release_dist\onnxruntime.dll"
    
    SetOutPath "$INSTDIR\resources"
    File /r "release_dist\resources"
    
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    
    CreateDirectory "$SMPROGRAMS\PDFbull"
    CreateShortcut "$SMPROGRAMS\PDFbull\PDFbull.lnk" "$INSTDIR\pdfbull.exe"
    CreateShortcut "$SMPROGRAMS\PDFbull\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\PDFbull.lnk" "$INSTDIR\pdfbull.exe"
    
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "DisplayName" "PDFbull"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "DisplayIcon" "$INSTDIR\pdfbull.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "Publisher" "SV-stark"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "NoRepair" 1
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\pdfbull.exe"
    Delete "$INSTDIR\pdfium.dll"
    Delete "$INSTDIR\onnxruntime.dll"
    Delete "$INSTDIR\Uninstall.exe"
    
    RMDir /r "$INSTDIR\resources"
    RMDir "$INSTDIR"
    
    Delete "$SMPROGRAMS\PDFbull\PDFbull.lnk"
    Delete "$SMPROGRAMS\PDFbull\Uninstall.lnk"
    RMDir "$SMPROGRAMS\PDFbull"
    
    Delete "$DESKTOP\PDFbull.lnk"
    
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull"
SectionEnd
