@echo off
echo Generatore automatico icone Tauri
echo =================================

if "%1"=="" (
    echo Uso: generate_icons.bat input_icon.png
    echo.
    echo Esempio: generate_icons.bat my_icon.png
    echo.
    echo NOTA: Se e' la prima volta, esegui prima setup.bat
    pause
    exit /b 1
)

if not exist "%1" (
    echo Errore: File %1 non trovato!
    pause
    exit /b 1
)

REM Verifica se le dipendenze sono installate
if not exist "libs\.dependencies_installed" (
    echo ⚠️  Dipendenze non trovate!
    echo Esegui prima: setup.bat
    echo.
    pause
    exit /b 1
)

echo Generazione icone da %1...
python generate_icons.py "%1"

echo.
echo Premere un tasto per continuare...
pause