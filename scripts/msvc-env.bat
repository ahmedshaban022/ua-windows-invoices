@echo off
REM Configure the MSVC toolchain (Visual Studio Build Tools 2022) for a shell.
REM Usage:  call D:\Coding\ua-windows-invoices\scripts\msvc-env.bat
REM
REM The Build Tools are installed under D:\VSBuildTools (not the default C: path)
REM so that they do not consume the limited free space on C:.

set "VCTOOLS=D:\VSBuildTools\VC\Tools\MSVC\14.44.35207"
set "WINSDK=C:\Program Files (x86)\Windows Kits\10"
set "SDKVER=10.0.26100.0"

if not exist "%VCTOOLS%\bin\Hostx64\x64\link.exe" (
  echo [msvc-env] Visual Studio Build Tools not found at %VCTOOLS%
  echo [msvc-env] Run: winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --installPath D:\VSBuildTools --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  exit /b 1
)

set "PATH=%VCTOOLS%\bin\Hostx64\x64;%WINSDK%\bin\%SDKVER%\x64;%PATH%"
set "LIB=%VCTOOLS%\lib\x64;%WINSDK%\Lib\%SDKVER%\ucrt\x64;%WINSDK%\Lib\%SDKVER%\um\x64"
set "INCLUDE=%VCTOOLS%\include;%WINSDK%\Include\%SDKVER%\ucrt;%WINSDK%\Include\%SDKVER%\um;%WINSDK%\Include\%SDKVER%\shared"

echo [msvc-env] MSVC %VCTOOLS% and Windows SDK %SDKVER% are on PATH.
