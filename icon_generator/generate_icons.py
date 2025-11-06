#!/usr/bin/env python3
"""
Script per generare automaticamente tutte le icone per Tauri da un singolo PNG.
Uso: python generate_icons.py input_icon.png
"""

import sys
import os
from pathlib import Path

# Aggiungi la cartella libs al path per le dipendenze locali
script_dir = Path(__file__).parent
libs_dir = script_dir / "libs"
if libs_dir.exists():
    sys.path.insert(0, str(libs_dir))

try:
    from PIL import Image, ImageDraw, ImageFilter
except ImportError:
    print("❌ Errore: Pillow non trovato!")
    print("💡 Esegui prima: python setup.py")
    print("   Oppure installa globalmente: pip install Pillow")
    sys.exit(1)

import argparse

def create_icon_with_background(image, size, background_color=(255, 255, 255, 0)):
    """Crea un'icona della dimensione specificata con uno sfondo opzionale"""
    # Crea una nuova immagine con sfondo
    icon = Image.new('RGBA', (size, size), background_color)
    
    # Ridimensiona l'immagine originale mantenendo le proporzioni
    image_copy = image.copy()
    
    # Per dimensioni piccole, usiamo un approccio in due passaggi per migliorare la qualità
    if size <= 150:
        # Prima ridimensiona a una dimensione intermedia più grande
        intermediate_size = max(size * 2, 256)
        if min(image_copy.size) > intermediate_size:
            image_copy.thumbnail((intermediate_size, intermediate_size), Image.Resampling.LANCZOS)
        
        # Poi applica ottimizzazioni per piccole dimensioni
        image_copy = optimize_for_small_sizes(image_copy, size)
    
    # Ridimensionamento finale
    image_copy.thumbnail((size, size), Image.Resampling.LANCZOS)
    
    # Calcola la posizione per centrare l'immagine
    x = (size - image_copy.width) // 2
    y = (size - image_copy.height) // 2
    
    # Incolla l'immagine centrata
    icon.paste(image_copy, (x, y), image_copy if image_copy.mode == 'RGBA' else None)
    
    return icon

def generate_png_icons(source_image, output_dir):
    """Genera tutte le icone PNG necessarie"""
    png_sizes = [
        # Icone standard
        (32, '32x32.png'),
        (128, '128x128.png'),
        (256, '128x128@2x.png'),  # 128x128@2x è in realtà 256x256
        (512, 'icon.png'),  # Icona principale ad alta risoluzione
        (512, 'logo.png'),  # Logo
        
        # Microsoft Store/UWP Logos - dimensioni ottimizzate per evitare pixelation
        (90, 'Square30x30Logo.png'),    # 30x30 logici -> 90x90 fisici (3x)
        (132, 'Square44x44Logo.png'),   # 44x44 logici -> 132x132 fisici (3x) 
        (213, 'Square71x71Logo.png'),   # 71x71 logici -> 213x213 fisici (3x)
        (267, 'Square89x89Logo.png'),   # 89x89 logici -> 267x267 fisici (3x)
        (321, 'Square107x107Logo.png'), # 107x107 logici -> 321x321 fisici (3x)
        (426, 'Square142x142Logo.png'), # 142x142 logici -> 426x426 fisici (3x)
        (450, 'Square150x150Logo.png'), # 150x150 logici -> 450x450 fisici (3x)
        (852, 'Square284x284Logo.png'), # 284x284 logici -> 852x852 fisici (3x)
        (930, 'Square310x310Logo.png'), # 310x310 logici -> 930x930 fisici (3x)
        (150, 'StoreLogo.png'),         # Store logo ottimizzato
    ]
    
    for size, filename in png_sizes:
        icon = create_icon_with_background(source_image, size)
        icon.save(output_dir / filename, 'PNG', optimize=True)
        print(f"✓ Generata: {filename} ({size}x{size})")

def generate_ico_icon(source_image, output_dir):
    """Genera l'icona ICO per Windows"""
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    icons = []
    
    for size in ico_sizes:
        icon = create_icon_with_background(source_image, size)
        # Converti in RGB se necessario (ICO non supporta sempre la trasparenza)
        if icon.mode == 'RGBA':
            # Crea uno sfondo bianco per ICO
            rgb_icon = Image.new('RGB', (size, size), (255, 255, 255))
            rgb_icon.paste(icon, mask=icon.split()[-1] if len(icon.split()) == 4 else None)
            icons.append(rgb_icon)
        else:
            icons.append(icon)
    
    # Salva come ICO multi-risoluzione
    icons[0].save(
        output_dir / 'icon.ico',
        format='ICO',
        sizes=[(icon.width, icon.height) for icon in icons],
        append_images=icons[1:]
    )
    print(f"✓ Generata: icon.ico (multi-risoluzione)")

def generate_icns_icon(source_image, output_dir):
    """Genera l'icona ICNS per macOS"""
    try:
        from PIL import Image
        import subprocess
        import tempfile
        
        # ICNS richiede dimensioni specifiche
        icns_sizes = [16, 32, 64, 128, 256, 512, 1024]
        
        # Crea una cartella temporanea per le immagini
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            iconset_path = temp_path / 'icon.iconset'
            iconset_path.mkdir()
            
            # Genera tutte le dimensioni richieste per ICNS
            for size in icns_sizes:
                icon = create_icon_with_background(source_image, size)
                
                # Nome file per iconset
                if size <= 32:
                    filename = f'icon_{size}x{size}.png'
                else:
                    filename = f'icon_{size}x{size}.png'
                
                # Salva anche la versione @2x per Retina
                icon.save(iconset_path / filename, 'PNG')
                if size <= 512:  # @2x versions
                    icon_2x = create_icon_with_background(source_image, size * 2)
                    icon_2x.save(iconset_path / f'icon_{size}x{size}@2x.png', 'PNG')
            
            # Usa iconutil di macOS per creare l'ICNS (solo su macOS)
            if sys.platform == 'darwin':
                try:
                    subprocess.run(['iconutil', '-c', 'icns', str(iconset_path), '-o', str(output_dir / 'icon.icns')], 
                                 check=True, capture_output=True)
                    print(f"✓ Generata: icon.icns (macOS)")
                    return
                except subprocess.CalledProcessError:
                    pass
            
            # Fallback: crea un PNG grande e rinominalo come ICNS
            fallback_icon = create_icon_with_background(source_image, 512)
            fallback_icon.save(output_dir / 'icon.icns', 'PNG')
            print(f"⚠ Generata: icon.icns (fallback PNG - per un vero ICNS usa macOS)")
            
    except Exception as e:
        print(f"⚠ Errore nella generazione ICNS: {e}")
        # Fallback: salva come PNG
        fallback_icon = create_icon_with_background(source_image, 512)
        fallback_icon.save(output_dir / 'icon.icns', 'PNG')
        print(f"⚠ Generata: icon.icns (fallback PNG)")

def optimize_for_small_sizes(image, target_size):
    """Ottimizza l'immagine per le dimensioni piccole"""
    # Per icone piccole, applica un leggero sharpening per migliorare la nitidezza
    if target_size <= 150:
        # Usa un algoritmo di ridimensionamento migliore per piccole dimensioni
        sharpened = image.filter(ImageFilter.UnsharpMask(radius=0.8, percent=200, threshold=3))
        return sharpened
    return image

def main():
    parser = argparse.ArgumentParser(description='Genera tutte le icone per Tauri da un PNG')
    parser.add_argument('input_png', help='File PNG di input (consigliato 512x512 o superiore)')
    parser.add_argument('-o', '--output', default='../src-tauri/icons', help='Cartella di output (default: ../src-tauri/icons)')
    parser.add_argument('--no-ico', action='store_true', help='Non generare il file ICO')
    parser.add_argument('--no-icns', action='store_true', help='Non generare il file ICNS')
    
    args = parser.parse_args()
    
    # Verifica che il file di input esista
    input_path = Path(args.input_png)
    if not input_path.exists():
        print(f"❌ Errore: File {input_path} non trovato!")
        sys.exit(1)
    
    # Crea la cartella di output
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    try:
        # Carica l'immagine sorgente
        print(f"📖 Caricamento immagine: {input_path}")
        source_image = Image.open(input_path)
        
        # Converti in RGBA se necessario
        if source_image.mode != 'RGBA':
            source_image = source_image.convert('RGBA')
        
        print(f"📏 Dimensioni originali: {source_image.width}x{source_image.height}")
        
        # Verifica che l'immagine sia abbastanza grande
        min_size = min(source_image.width, source_image.height)
        if min_size < 256:
            print("⚠ Attenzione: L'immagine è piccola. Per risultati migliori usa un'immagine di almeno 512x512 pixel.")
        
        print(f"🎨 Generazione icone in: {output_dir}")
        
        # Genera le icone PNG
        print("\n📱 Generazione icone PNG...")
        generate_png_icons(source_image, output_dir)
        
        # Genera l'icona ICO per Windows
        if not args.no_ico:
            print("\n🪟 Generazione icona ICO per Windows...")
            generate_ico_icon(source_image, output_dir)
        
        # Genera l'icona ICNS per macOS
        if not args.no_icns:
            print("\n🍎 Generazione icona ICNS per macOS...")
            generate_icns_icon(source_image, output_dir)
        
        print(f"\n✅ Completato! Tutte le icone sono state generate in: {output_dir}")
        print("\n💡 Suggerimenti:")
        print("   - Ricompila l'app Tauri per vedere le nuove icone")
        print("   - Su Windows potresti dover pulire la cache delle icone")
        print("   - Su macOS l'icona potrebbe aggiornarsi dopo un riavvio del Finder")
        
    except Exception as e:
        print(f"❌ Errore: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()