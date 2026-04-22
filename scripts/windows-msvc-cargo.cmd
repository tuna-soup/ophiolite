@echo off
setlocal

if "%~1"=="" (
  echo usage: windows-msvc-cargo.cmd ^<cargo-args...^>
  echo.
  echo Required environment variables:
  echo   OPHIOLITE_SQLITE_INCLUDE   path containing sqlite3.h
  echo   OPHIOLITE_SQLITE_LIB_DIR   path containing sqlite3.lib / libsqlite3.a
  echo.
  echo Optional environment variables:
  echo   OPHIOLITE_SQLITE_BIN_DIR   path containing sqlite3.exe
  echo   OPHIOLITE_VSDEVCMD         path to VsDevCmd.bat
  exit /b 2
)

if "%OPHIOLITE_VSDEVCMD%"=="" (
  set "OPHIOLITE_VSDEVCMD=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
)

if not exist "%OPHIOLITE_VSDEVCMD%" (
  echo VsDevCmd not found: "%OPHIOLITE_VSDEVCMD%"
  exit /b 2
)

if "%OPHIOLITE_SQLITE_INCLUDE%"=="" (
  echo OPHIOLITE_SQLITE_INCLUDE is required
  exit /b 2
)

if "%OPHIOLITE_SQLITE_LIB_DIR%"=="" (
  echo OPHIOLITE_SQLITE_LIB_DIR is required
  exit /b 2
)

if not exist "%OPHIOLITE_SQLITE_INCLUDE%\sqlite3.h" (
  echo sqlite3.h not found under "%OPHIOLITE_SQLITE_INCLUDE%"
  exit /b 2
)

set "OPHIOLITE_SQLITE_LIBRARY="
if exist "%OPHIOLITE_SQLITE_LIB_DIR%\sqlite3.lib" (
  set "OPHIOLITE_SQLITE_LIBRARY=%OPHIOLITE_SQLITE_LIB_DIR%\sqlite3.lib"
) else if exist "%OPHIOLITE_SQLITE_LIB_DIR%\libsqlite3.lib" (
  set "OPHIOLITE_SQLITE_LIBRARY=%OPHIOLITE_SQLITE_LIB_DIR%\libsqlite3.lib"
) else if exist "%OPHIOLITE_SQLITE_LIB_DIR%\libsqlite3.a" (
  set "OPHIOLITE_SQLITE_LIBRARY=%OPHIOLITE_SQLITE_LIB_DIR%\libsqlite3.a"
)

if "%OPHIOLITE_SQLITE_LIBRARY%"=="" (
  echo sqlite3 library not found under "%OPHIOLITE_SQLITE_LIB_DIR%"
  echo expected one of: sqlite3.lib, libsqlite3.lib, libsqlite3.a
  exit /b 2
)

call "%OPHIOLITE_VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b %errorlevel%

set "PATH=%PATH:C:\msys64\ucrt64\bin;=%"
set "PATH=%PATH:C:\Qt\6.8.1\mingw_64\bin;=%"
set "PATH=%PATH:C:\Strawberry\c\bin;=%"
set "PATH=%PATH:C:\Strawberry\perl\site\bin;=%"
set "PATH=%PATH:C:\Strawberry\perl\bin;=%"

if not "%OPHIOLITE_SQLITE_BIN_DIR%"=="" (
  if not exist "%OPHIOLITE_SQLITE_BIN_DIR%\sqlite3.exe" (
    echo sqlite3.exe not found under "%OPHIOLITE_SQLITE_BIN_DIR%"
    exit /b 2
  )
  set "PATH=%OPHIOLITE_SQLITE_BIN_DIR%;%PATH%"
)

set "DEP_SQLITE3_INCLUDE=%OPHIOLITE_SQLITE_INCLUDE%"
set "DEP_SQLITE3_LIB_DIR=%OPHIOLITE_SQLITE_LIB_DIR%"
set "SQLite3_INCLUDE_DIR=%OPHIOLITE_SQLITE_INCLUDE%"
set "SQLite3_LIBRARY=%OPHIOLITE_SQLITE_LIBRARY%"
set "CMAKE_INCLUDE_PATH=%OPHIOLITE_SQLITE_INCLUDE%"
set "CMAKE_LIBRARY_PATH=%OPHIOLITE_SQLITE_LIB_DIR%"
set "PKG_CONFIG_EXECUTABLE="
set "LIB=%OPHIOLITE_SQLITE_LIB_DIR%;%LIB%"

cargo %*
exit /b %errorlevel%
