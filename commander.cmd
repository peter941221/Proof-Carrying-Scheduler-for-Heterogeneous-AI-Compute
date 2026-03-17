@echo off
setlocal
powershell -ExecutionPolicy Bypass -File "%~dp0LangGraph-Commander\start.ps1" %*
