@echo off
setlocal

:: Gets working dir
set HERE=%~dp0

:: Add this directory to the PATH so windows can find DLLs
set PATH=%HERE%;%PATH%

"%HERE%cosmos_server.exe" %*

endlocal
