@echo off
setlocal

:: Gets working dir
set HERE=%~dp0

"%HERE%cosmos_server.exe" %*

endlocal
