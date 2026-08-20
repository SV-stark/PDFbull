!include "MUI2.nsh"

Name "PDFbull"
OutFile "PDFbull-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\PDFbull"
RequestExecutionLevel user

; Branding and Graphics
BrandingText "PDFbull - Pure-Rust PDF & Neural OCR"
!define MUI_ICON "PDFbull.ico"
!define MUI_UNICON "PDFbull.ico"

!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "installer_assets\header.bmp"
!define MUI_HEADERIMAGE_RIGHT

!define MUI_WELCOMEFINISHPAGE_BITMAP "installer_assets\welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "installer_assets\welcome.bmp"

; Welcome Page Customization
!define MUI_WELCOMEPAGE_TITLE "Welcome to PDFbull Setup"
!define MUI_WELCOMEPAGE_TEXT "PDFbull is a high-performance PDF reader and workspace powered by pure-Rust rendering (zpdf) and built-in neural OCR (rten).\r\n\r\nThis setup wizard will install PDFbull along with Latin and Devanagari neural OCR models.\r\n\r\nClick Next to continue."

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

; Finish Page Customization
!define MUI_FINISHPAGE_RUN "$INSTDIR\pdfbull.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch PDFbull Now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

Section "Install"
    SetShellVarContext current
    SetOutPath "$INSTDIR"
    
    File "release_dist\pdfbull.exe"
    File /nonfatal "PDFbull.ico"
    
    SetOutPath "$INSTDIR\models"
    File /nonfatal /r "models\*.*"
    SetOutPath "$INSTDIR"
    
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    
    CreateDirectory "$SMPROGRAMS\PDFbull"
    CreateShortcut "$SMPROGRAMS\PDFbull\PDFbull.lnk" "$INSTDIR\pdfbull.exe" "" "$INSTDIR\PDFbull.ico"
    CreateShortcut "$SMPROGRAMS\PDFbull\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\PDFbull.lnk" "$INSTDIR\pdfbull.exe" "" "$INSTDIR\PDFbull.ico"
    
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "DisplayName" "PDFbull"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "DisplayIcon" "$INSTDIR\pdfbull.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "Publisher" "SV-stark"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull" "NoRepair" 1
SectionEnd

Section "Uninstall"
    SetShellVarContext current
    Delete "$INSTDIR\pdfbull.exe"
    Delete "$INSTDIR\PDFbull.ico"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir /r "$INSTDIR\models"
    RMDir "$INSTDIR"
    
    Delete "$SMPROGRAMS\PDFbull\PDFbull.lnk"
    Delete "$SMPROGRAMS\PDFbull\Uninstall.lnk"
    RMDir "$SMPROGRAMS\PDFbull"
    
    Delete "$DESKTOP\PDFbull.lnk"
    
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PDFbull"
SectionEnd
