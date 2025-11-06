# 🎨 Generatore Automatico Icone Tauri

Questo script Python genera automaticamente tutte le icone necessarie per un'applicazione Tauri partendo da un singolo file PNG.

## 📋 Requisiti

- Python 3.6 o superiore
- Pillow (PIL) per la manipolazione delle immagini

## 🚀 Setup Iniziale (Una tantum)

### Metodo 1: Setup automatico (Windows)
```cmd
setup.bat
```

### Metodo 2: Setup manuale (multipiattaforma)
```bash
python setup.py
```

Questo installerà Pillow nella cartella `libs/` locale, così non serve installarlo globalmente su ogni computer.

## 📝 Utilizzo

### Metodo 1: Script Python (multipiattaforma)
```bash
python generate_icons.py input_icon.png
```

### Metodo 2: File batch (solo Windows)
```cmd
generate_icons.bat input_icon.png
```

**Nota**: Se è la prima volta che usi lo script, esegui prima il setup!

## 🎯 Opzioni

```bash
# Specifica cartella di output personalizzata
python generate_icons.py my_icon.png -o ../custom_icons_folder/

# Salta la generazione del file ICO (Windows)
python generate_icons.py my_icon.png --no-ico

# Salta la generazione del file ICNS (macOS)
python generate_icons.py my_icon.png --no-icns

# Aiuto completo
python generate_icons.py -h
```

## 📁 File Generati

Lo script genera automaticamente tutti questi file nella cartella `../src-tauri/icons/`:

### Icone PNG Standard
- `32x32.png` - Icona piccola
- `128x128.png` - Icona media
- `128x128@2x.png` - Icona media Retina (256x256)
- `icon.png` - Icona principale (512x512)
- `logo.png` - Logo (512x512)

### Icone Windows Store/UWP
- `Square30x30Logo.png`
- `Square44x44Logo.png`
- `Square71x71Logo.png`
- `Square89x89Logo.png`
- `Square107x107Logo.png`
- `Square142x142Logo.png`
- `Square150x150Logo.png`
- `Square284x284Logo.png`
- `Square310x310Logo.png`
- `StoreLogo.png`

### Icone Sistema
- `icon.ico` - Icona Windows (multi-risoluzione)
- `icon.icns` - Icona macOS (richiede macOS per la generazione ottimale)

## 💡 Consigli per l'Immagine di Input

- **Dimensioni**: Usa un'immagine di almeno **1024x1024 pixel** per risultati ottimali (migliore qualità per StoreLogo)
- **Formato**: PNG con trasparenza (RGBA) preferito
- **Design**: Assicurati che l'icona sia leggibile anche a dimensioni piccole (32x32)
- **Margini**: Lascia un piccolo margine attorno all'elemento principale
- **Colori**: Usa colori contrastanti per buona visibilità
- **Dettagli**: Evita dettagli troppo fini che si perderebbero nelle icone piccole

## 🎨 Miglioramenti Qualità

Lo script ora include:
- **Ridimensionamento intelligente**: Usa tecniche avanzate per piccole dimensioni
- **Sharpening automatico**: Migliora la nitidezza delle icone piccole
- **Dimensioni ottimizzate**: Icone Microsoft Store a risoluzione 3x per evitare pixelation
- **Algoritmo in due passaggi**: Ridimensionamento intermedio per migliore qualità

## 🎯 Vantaggi del Sistema di Dipendenze Locali

- ✅ **Portabile**: Non serve installare nulla globalmente
- ✅ **Isolato**: Non interferisce con altre installazioni Python
- ✅ **Semplice**: Una sola cartella da copiare su altri computer
- ✅ **Veloce**: Setup una tantum, poi funziona ovunque

## 🔧 Risoluzione Problemi

### Errore "Pillow not found"
```bash
# Esegui il setup per installare le dipendenze locali
python setup.py
```

### Su un nuovo computer
1. Copia l'intera cartella `icon_generator/`
2. Esegui `setup.bat` (Windows) o `python setup.py` (multipiattaforma)
3. Usa normalmente gli script

### ICNS non generato correttamente
- Su macOS: Automaticamente usa `iconutil` per risultati ottimali
- Su Windows/Linux: Genera un fallback PNG rinominato come .icns

### Le icone non si aggiornano nell'app
1. Ricompila completamente l'app Tauri
2. Su Windows: Pulisci la cache delle icone del sistema
3. Su macOS: Riavvia il Finder o fai logout/login

## 📸 Esempio

```bash
# Partendo da un file my_awesome_icon.png nella cartella icon_generator
python generate_icons.py my_awesome_icon.png

# Risultato: 16 file icona generati automaticamente in ../src-tauri/icons/
```

---

**Nota**: Questo è uno strumento di sviluppo. I file generati saranno integrati nell'app Tauri, ma la cartella `icon_generator` non è necessaria per la distribuzione finale.