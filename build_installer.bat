@echo off
setlocal
echo ========================================================
echo   Building FocusGrow Windows Installer with Inno Setup 6
echo ========================================================

:: 1. Build optimized release binary with Cargo
echo [1/4] Building FocusGrow release binary with Cargo...
powershell -NoProfile -Command "Stop-Process -Name FocusGrow,ytmpx_ui,ytmpx-core -Force -ErrorAction SilentlyContinue" >nul 2>&1
call cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo release build failed. Installer creation aborted.
    exit /b 1
)

:: 2. Stage binary and companion extensions into bin\
echo [2/4] Staging FocusGrow binary and companion extensions into bin\...
if not exist "bin" mkdir "bin"
copy /Y "target\release\FocusGrow.exe" "bin\FocusGrow.exe" >nul

:: FocusGrow Browser Extension (Restricted Sites & Cooldown Guard)
if exist "F:\Android Project\FocusGrow - webview\extension" (
    if not exist "bin\extension-focusgrow" mkdir "bin\extension-focusgrow"
    xcopy /E /I /Y "F:\Android Project\FocusGrow - webview\extension\*" "bin\extension-focusgrow\" >nul
)

:: YouTube Music Browser Extension (Media sync companion)
if exist "F:\Extension\ytmpx\apps\chrome\dist" (
    if not exist "bin\extension-ytmpx" mkdir "bin\extension-ytmpx"
    xcopy /E /I /Y "F:\Extension\ytmpx\apps\chrome\dist\*" "bin\extension-ytmpx\" >nul
)
if exist "bin\ytmpx-server" rmdir /S /Q "bin\ytmpx-server" >nul 2>&1

:: 3. Locate Inno Setup Compiler
echo [3/4] Locating Inno Setup 6 Compiler...
set "ISCC_PATH=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if not exist "%ISCC_PATH%" (
    set "ISCC_PATH=C:\Program Files\Inno Setup 6\ISCC.exe"
)

if not exist "%ISCC_PATH%" (
    echo [ERROR] Inno Setup 6 ISCC.exe was not found in Program Files.
    echo Please install Inno Setup 6 from https://jrsoftware.org/isinfo.php
    exit /b 1
)

:: 4. Compile Installer into dist\
echo [4/4] Compiling installer package with Inno Setup 6...
if not exist "dist" mkdir "dist"
"%ISCC_PATH%" "installer.iss"
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Inno Setup compilation failed.
    exit /b 1
)

echo.
echo ========================================================
echo  [SUCCESS] FocusGrow Installer created successfully in dist\
echo ========================================================
dir "dist\*.exe"
