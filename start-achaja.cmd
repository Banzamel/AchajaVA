@echo off
rem Uruchamia nasluch Achai; nasluch otwiera okno Claude (Achaja) i steruje nim glosem.
rem   start-achaja.cmd             nowa rozmowa
rem   start-achaja.cmd --continue  wznow ostatnia rozmowe
title Achaja - nasluch
cd /d "%~dp0"
set "PYTHONPATH=%~dp0src"
".venv\Scripts\python.exe" -m achaja.voice.listener %*
pause
