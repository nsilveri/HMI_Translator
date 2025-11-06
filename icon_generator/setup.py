#!/usr/bin/env python3
"""
Setup script per installare le dipendenze localmente nella cartella libs
"""

import subprocess
import sys
import os
from pathlib import Path

def install_dependencies():
    """Installa le dipendenze nella cartella libs locale"""
    script_dir = Path(__file__).parent
    libs_dir = script_dir / "libs"
    
    print("🔧 Setup generatore icone Tauri")
    print("=" * 40)
    
    # Crea la cartella libs se non esiste
    libs_dir.mkdir(exist_ok=True)
    
    # Installa Pillow nella cartella libs
    try:
        print("📦 Installazione Pillow nella cartella libs...")
        subprocess.run([
            sys.executable, "-m", "pip", "install", 
            "--target", str(libs_dir),
            "--upgrade",
            "Pillow>=10.0.0"
        ], check=True)
        print("✅ Pillow installato con successo!")
        
        # Crea un file marker per indicare che le dipendenze sono installate
        marker_file = libs_dir / ".dependencies_installed"
        marker_file.write_text("Dependencies installed successfully")
        
        print(f"✅ Dipendenze installate in: {libs_dir}")
        print("\n💡 Ora puoi usare generate_icons.py senza installazioni globali!")
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Errore durante l'installazione: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Errore generico: {e}")
        sys.exit(1)

if __name__ == "__main__":
    install_dependencies()