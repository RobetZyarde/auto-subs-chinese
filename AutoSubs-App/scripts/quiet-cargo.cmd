@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0quiet-cargo.ps1" %*
