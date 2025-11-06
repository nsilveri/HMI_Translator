@echo off
echo Setup Generatore Icone Tauri
echo =============================
echo.

echo Installazione dipendenze locali...
python setup.py

if %ERRORLEVEL% EQU 0 (
    echo.
    echo =============================
    echo ✅ Setup completato!
    echo.
    echo Ora puoi usare:
    echo   generate_icons.bat my_icon.png
    echo   oppure
    echo   python generate_icons.py my_icon.png
    echo.
) else (
    echo.
    echo ❌ Errore durante il setup!
    echo Verifica di avere Python installato.
)

echo.
echo Premere un tasto per continuare...
pause >nul