@echo off
rem rustc invokes this wrapper with an @response-file for the final link.
rem While rustc's temp object files still exist we also produce the
rem static-PIE variant (crt1.o -> rcrt1.o) via rcrtlink.ps1, then forward
rem the original invocation to rust-lld.
set ARG1=%~1
if "%ARG1:~0,1%"=="@" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0rcrtlink.ps1" "%ARG1:~1%" > "%~dp0rcrt-link.log" 2>&1
)
C:\Users\feng.han\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe -flavor gnu %*
exit /b %ERRORLEVEL%
