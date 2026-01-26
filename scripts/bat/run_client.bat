@echo off
setlocal

:: Gets working dir
set HERE=%~dp0

"%HERE%cosmos_client.exe" %*

endlocal
