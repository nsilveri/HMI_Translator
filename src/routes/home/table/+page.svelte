<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';

  let languages = [];
  let tableName = '';
  let projectInfo = null;
  let foundTranslationFiles = [];

  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';
  let showAddLanguageModal = false;
  let newLanguageCode = '';
  let newLanguageName = '';
  let loading = false;
  let showKeysModal = false;
  let foundKeys = [];
  
  // Card display settings (simili alla home)
  let cardWidth = 180; // px
  let cardHeight = 170; // px
  let imageHeight = 90; // px
  const defaultImage = "https://placehold.co/160x90?text=Language";

  onMount(async () => {
    const urlParams = $page.url.searchParams;
    tableName = urlParams.get('table') || '';
    console.log('Table name:', tableName);

    if (!tableName) {
      console.log('No table name in URL');
      return;
    }

    await loadProjectData();
  });

  async function loadProjectData() {
    try {
      // Carica le informazioni del progetto e le lingue
      projectInfo = await invoke('get_table_info', { tableName: tableName });
      languages = await invoke('get_project_languages', { projectName: tableName });
      console.log('Loaded languages:', languages);
      console.log('Project info:', projectInfo);
      
      // Se abbiamo il percorso del progetto, scansiona i file di traduzione
      if (projectInfo && projectInfo.path) {
        console.log('Scanning directory for translation files:', projectInfo.path);
        try {
          foundTranslationFiles = await invoke('get_translation_files_in_directory', { 
            directoryPath: projectInfo.path 
          });
          console.log('Found translation files:', foundTranslationFiles);
        } catch (e) {
          console.error('Errore scansione file di traduzione:', e);
          foundTranslationFiles = [];
        }
      } else {
        console.log('No project path available, projectInfo:', projectInfo);
        foundTranslationFiles = [];
      }
    } catch (e) {
      console.error('Errore caricamento dati progetto:', e);
      toastMsg = 'Errore nel caricamento del progetto: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }

  async function addLanguage() {
    if (!newLanguageCode.trim() || !newLanguageName.trim()) {
      toastMsg = 'Inserisci codice e nome della lingua';
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2500);
      return;
    }

    loading = true;
    try {
      const result = await invoke('add_language_to_project', { 
        project_name: tableName, 
        language_code: newLanguageCode.trim().toLowerCase(), 
        language_name: newLanguageName.trim() 
      });
      toastMsg = result;
      toastType = 'success';
      await loadProjectData(); // Ricarica i dati
      closeAddLanguageModal();
    } catch (e) {
      toastMsg = 'Errore nell\'aggiunta della lingua: ' + e;
      toastType = 'error';
    }
    loading = false;
    showToast = true;
    setTimeout(() => { showToast = false; }, 2500);
  }

  function openAddLanguageModal() {
    showAddLanguageModal = true;
    newLanguageCode = '';
    newLanguageName = '';
  }

  function closeAddLanguageModal() {
    showAddLanguageModal = false;
    newLanguageCode = '';
    newLanguageName = '';
  }

  const importTranslationFile = async () => {
    try {
      // Uso l'API del browser per selezionare il file
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.xml,.eng,.ita';
      
      input.onchange = async (event) => {
        const file = event.target.files[0];
        if (file) {
          loading = true;
          try {
            const text = await file.text();
            
            // Auto-rileva la lingua dall'estensione del file
            let languageCode = '';
            const fileName = file.name.toLowerCase();
            if (fileName.endsWith('.eng')) {
              languageCode = 'en';
            } else if (fileName.endsWith('.ita')) {
              languageCode = 'it';
            } else {
              // Per file .xml, proviamo a rilevare dalla struttura o chiediamo all'utente
              // Per ora usiamo un prompt semplice
              languageCode = prompt('Inserisci il codice lingua per questo file (es: en, it, fr):');
              if (!languageCode) {
                toastMsg = 'Importazione annullata: codice lingua richiesto';
                toastType = 'error';
                showToast = true;
                setTimeout(() => { showToast = false; }, 3000);
                loading = false;
                return;
              }
            }
            
            await invoke('import_translation_file', {
              tableName: tableName,
              languageCode: languageCode.toLowerCase(),
              xmlContent: text
            });
            
            toastMsg = `File ${file.name} importato con successo per la lingua ${languageCode}!`;
            toastType = 'success';
            showToast = true;
            setTimeout(() => { showToast = false; }, 3000);
            
            // Ricarica i dati del progetto
            await loadProjectData();
          } catch (error) {
            toastMsg = 'Errore nell\'importazione del file: ' + error;
            toastType = 'error';
            showToast = true;
            setTimeout(() => { showToast = false; }, 3000);
          }
          loading = false;
        }
      };
      
      input.click();
    } catch (error) {
      console.error('Errore nell\'importazione del file:', error);
    }
  };
  
  async function importFoundFile(fileInfo) {
    loading = true;
    
    try {
      // Leggi il file dalla directory del progetto
      const fs = await import('@tauri-apps/plugin-fs');
      const text = await fs.readTextFile(fileInfo.file_path);
      
      await invoke('import_translation_file', {
        tableName: tableName,
        languageCode: fileInfo.language_code,
        xmlContent: text
      });
      
      toastMsg = `File ${fileInfo.file_name} importato con successo per ${fileInfo.language_name}!`;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      // Ricarica i dati del progetto
      await loadProjectData();
      
    } catch (e) {
      console.error('Errore importazione file trovato:', e);
      toastMsg = 'Errore importazione file: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  async function searchProjectFiles() {
    loading = true;
    
    try {
      // Prima prova a cercare automaticamente nella directory del progetto
      if (projectInfo && projectInfo.path) {
        console.log('Ricerca automatica nella directory:', projectInfo.path);
        foundTranslationFiles = await invoke('get_translation_files_in_directory', { 
          directoryPath: projectInfo.path 
        });
        
        if (foundTranslationFiles.length > 0) {
          toastMsg = `Trovati ${foundTranslationFiles.length} file di traduzione nella directory del progetto!`;
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
          loading = false;
          return;
        }
      }
      
      // Se non trova file automaticamente, chiede di selezionarli manualmente
      toastMsg = 'Nessun file di traduzione trovato automaticamente. Seleziona i file manualmente.';
      toastType = 'info';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
      
      // Apre dialog per selezione multipla
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: false,
        multiple: true,
        filters: [{
          name: 'File di traduzione',
          extensions: ['xml', 'ita', 'eng', 'fra', 'fre', 'deu', 'ger', 'esp', 'spa']
        }]
      });
      
      if (selected && Array.isArray(selected)) {
        // Processa i file selezionati
        const manualFiles = [];
        
        for (const filePath of selected) {
          const fileName = filePath.split(/[/\\]/).pop() || '';
          const fileNameLower = fileName.toLowerCase();
          
          // Mappa delle estensioni alle lingue
          let languageName = 'Sconosciuto';
          let languageCode = 'unknown';
          
          if (fileNameLower.endsWith('.ita')) {
            languageName = 'Italiano';
            languageCode = 'it';
          } else if (fileNameLower.endsWith('.eng')) {
            languageName = 'English';
            languageCode = 'en';
          } else if (fileNameLower.endsWith('.fra') || fileNameLower.endsWith('.fre')) {
            languageName = 'Français';
            languageCode = 'fr';
          } else if (fileNameLower.endsWith('.deu') || fileNameLower.endsWith('.ger')) {
            languageName = 'Deutsch';
            languageCode = 'de';
          } else if (fileNameLower.endsWith('.esp') || fileNameLower.endsWith('.spa')) {
            languageName = 'Español';
            languageCode = 'es';
          } else if (fileNameLower.endsWith('.xml')) {
            // Per i file XML, chiediamo la lingua
            const langCode = prompt(`Inserisci il codice lingua per il file ${fileName} (es: en, it, fr):`);
            if (langCode) {
              languageCode = langCode.toLowerCase();
              languageName = getLanguageNameFromCode(languageCode);
            }
          }
          
          if (languageCode !== 'unknown') {
            manualFiles.push({
              file_name: fileName,
              file_path: filePath,
              language_name: languageName,
              language_code: languageCode
            });
          }
        }
        
        foundTranslationFiles = manualFiles;
        
        if (foundTranslationFiles.length > 0) {
          toastMsg = `Selezionati ${foundTranslationFiles.length} file di traduzione!`;
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        } else {
          toastMsg = 'Nessun file di traduzione valido selezionato.';
          toastType = 'warning';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        }
      }
      
    } catch (e) {
      console.error('Errore nella ricerca dei file:', e);
      toastMsg = 'Errore nella ricerca dei file: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  function getLanguageNameFromCode(code) {
    const languageMap = {
      'it': 'Italiano',
      'en': 'English', 
      'fr': 'Français',
      'de': 'Deutsch',
      'es': 'Español'
    };
    return languageMap[code] || code.toUpperCase();
  }
  
  async function findProjectKeys() {
    if (!projectInfo || !projectInfo.path) {
      toastMsg = 'Percorso del progetto non disponibile';
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      return;
    }
    
    loading = true;
    
    try {
      foundKeys = await invoke('find_keys_in_project', { 
        directoryPath: projectInfo.path,
        projectName: tableName
      });
      
      showKeysModal = true;
      
      toastMsg = `Trovate ${foundKeys.length} chiavi uniche nei file .hmiscr`;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
    } catch (e) {
      console.error('Errore nella ricerca delle chiavi:', e);
      toastMsg = 'Errore nella ricerca delle chiavi: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  async function importFoundKeys() {
    if (foundKeys.length === 0) {
      toastMsg = 'Nessuna chiave da importare';
      toastType = 'warning';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      return;
    }
    
    loading = true;
    
    try {
      const result = await invoke('import_project_keys', { 
        projectName: tableName,
        keys: foundKeys
      });
      
      toastMsg = result;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      showKeysModal = false;
      
    } catch (e) {
      console.error('Errore importazione chiavi:', e);
      toastMsg = 'Errore importazione chiavi: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
</script>

<div class="min-h-screen flex flex-col" style="background: linear-gradient(135deg, #c9ffe7 0%, #e9e9ff 70%, #dcecff 100%);">
  
  <!-- TOAST NOTIFICATIONS -->
  {#if showToast}
    <div class="fixed bottom-8 right-8 z-50 px-6 py-3 rounded shadow-lg animate-fadein font-semibold text-white"
      style="background-color: {toastType === 'success' ? '#22c55e' : '#ef4444'};">
      {toastMsg}
    </div>
  {/if}

  <!-- HEADER CONTENT -->
  <header class="w-full pt-5 px-5 fixed top-0 left-0 right-0 z-10 bg-transparent">
    <div class="w-full bg-white/50 backdrop-blur-sm rounded-lg border border-black/50 p-4 sm:p-2 shadow-lg flex items-center justify-between">
      <button class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded" on:click={() => window.history.back()}>
        ← {$_('home.back')}
      </button>
      
      <div class="text-center flex-1">
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">Progetto: {tableName}</h1>
        <p class="text-gray-700 text-sm">Lingue del progetto</p>
      </div>
      
      <div class="flex gap-2 items-center">
        <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={importTranslationFile} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"></path>
          </svg>
          {loading ? 'Importando...' : 'Importa File'}
        </button>
        <button class="bg-purple-500 hover:bg-purple-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={searchProjectFiles} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
          Cerca nel progetto
        </button>
        <button class="bg-orange-500 hover:bg-orange-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={findProjectKeys} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"></path>
          </svg>
          Trova Chiavi
        </button>
        <button class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={openAddLanguageModal}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
          </svg>
          Aggiungi Lingua
        </button>
      </div>
    </div>
  </header>

  <!-- MAIN CONTENT -->
  <main class="flex-grow pt-5 px-5 mb-8" style="margin-top: 6rem; margin-bottom: 2rem;">
    <div class="w-full h-full overflow-y-auto pb-20" style="scrollbar-width: thin;">
      
      <!-- Sezione file trovati nel progetto -->
      {#if foundTranslationFiles.length > 0}
        <div class="mb-6">
          <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
            <svg class="w-5 h-5 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
            File di traduzione trovati nel progetto
          </h3>
          <!-- Linea separatrice sopra -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mb-4"></div>
          <div class="flex flex-wrap justify-center gap-4">
            {#each foundTranslationFiles as fileInfo}
              <div class="bg-yellow-100/90 backdrop-blur-sm rounded-lg border border-yellow-300/50 p-2 text-center shadow-lg"
                style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                
                <div class="rounded-lg w-full bg-yellow-200 mb-2 flex items-center justify-center"
                  style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                  <svg class="w-12 h-12 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                  </svg>
                </div>
                
                <h2 class="text-base font-semibold text-yellow-800 mb-1 truncate">{fileInfo.language_name}</h2>
                <p class="text-xs text-yellow-600 mb-2 truncate" title={fileInfo.file_name}>{fileInfo.file_name}</p>
                
                <div class="flex justify-center">
                  <button 
                    on:click={() => importFoundFile(fileInfo)}
                    disabled={loading}
                    class="bg-green-500 hover:bg-green-700 disabled:bg-gray-400 text-white font-bold py-1 px-3 rounded text-xs flex items-center gap-1">
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"></path>
                    </svg>
                    Importa
                  </button>
                </div>
              </div>
            {/each}
          </div>
          <!-- Linea separatrice sotto -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mt-4"></div>
        </div>
      {/if}
      
      <!-- Sezione lingue configurate -->
      {#if languages.length > 0}
        <div class="mb-6">
          <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
            <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
            </svg>
            Lingue configurate
          </h3>
          <!-- Linea separatrice sopra -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-blue-400 to-transparent mb-4"></div>
          <div class="flex flex-wrap justify-center gap-4">
            {#each languages as language}
              <div class="bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-2 text-center shadow-lg"
                style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                
                <img src={defaultImage} alt="Lingua {language.name}"
                  class="rounded-lg w-full object-cover mb-2"
                  style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;" />
                
                <h2 class="text-base font-semibold text-gray-900 mb-2 truncate">{language.name} ({language.code})</h2>
                
                <div class="flex justify-center gap-2">
                    <a href="/home/table/translations?table={encodeURIComponent(tableName)}&lang={encodeURIComponent(language.code)}" 
                       class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-1 px-2 rounded text-xs flex items-center gap-1">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                      </svg>
                      Traduci
                    </a>
                    <button class="bg-red-500 hover:bg-red-700 text-white font-bold py-1 px-2 rounded text-xs flex items-center gap-1">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                      </svg>
                      Elimina
                    </button>
                </div>
              </div>
            {/each}
          </div>
          <!-- Linea separatrice sotto -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-blue-400 to-transparent mt-4"></div>
        </div>
      {/if}

      {#if languages.length === 0 && foundTranslationFiles.length === 0}
        <div class="text-center text-gray-500 py-8">
          <p class="mb-4">Nessuna lingua configurata per questo progetto.</p>
          <button class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded" on:click={openAddLanguageModal}>
            Aggiungi Prima Lingua
          </button>
        </div>
      {/if}
    </div>
  </main>

  <!-- ADD LANGUAGE MODAL -->
  {#if showAddLanguageModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div class="bg-white rounded-lg shadow-lg p-6 w-full max-w-sm">
        <h2 class="text-lg font-semibold mb-4">Aggiungi Nuova Lingua</h2>
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Codice Lingua (es: it, en, fr)</label>
          <input 
            type="text" 
            bind:value={newLanguageCode} 
            placeholder="it" 
            class="w-full border rounded px-3 py-2 text-sm"
            maxlength="5"
          />
        </div>
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Nome Lingua</label>
          <input 
            type="text" 
            bind:value={newLanguageName} 
            placeholder="Italiano" 
            class="w-full border rounded px-3 py-2 text-sm"
          />
        </div>
        <div class="flex justify-end gap-3">
          <button 
            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded" 
            on:click={closeAddLanguageModal}
            disabled={loading}
          >
            Annulla
          </button>
          <button 
            class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded" 
            on:click={addLanguage}
            disabled={loading}
          >
            {loading ? 'Aggiunta...' : 'Aggiungi'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- KEYS MODAL -->
  {#if showKeysModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div class="bg-white rounded-lg shadow-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-lg font-semibold flex items-center gap-2">
            <svg class="w-5 h-5 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"></path>
            </svg>
            Chiavi trovate nel progetto ({foundKeys.length})
          </h2>
          <button 
            class="text-gray-500 hover:text-gray-700"
            on:click={() => showKeysModal = false}
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto">
          {#if foundKeys.length === 0}
            <p class="text-gray-500 text-center py-8">Nessuna chiave trovata nei file .hmiscr</p>
          {:else}
            <div class="space-y-2">
              {#each foundKeys as key}
                <div class="bg-gray-50 rounded px-3 py-2 border border-gray-200 font-mono text-sm">
                  {key}
                </div>
              {/each}
            </div>
          {/if}
        </div>
        
        <div class="flex justify-end gap-3 mt-4 pt-4 border-t">
          <button 
            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded" 
            on:click={() => showKeysModal = false}
            disabled={loading}
          >
            Chiudi
          </button>
          {#if foundKeys.length > 0}
            <button 
              class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
              on:click={importFoundKeys}
              disabled={loading}
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"></path>
              </svg>
              {loading ? 'Importando...' : `Importa ${foundKeys.length} Chiavi`}
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- TOAST -->
  {#if showToast}
    <div class="fixed bottom-8 right-8 z-50 px-6 py-3 rounded shadow-lg animate-fadein font-semibold text-white"
      style="background-color: {toastType === 'success' ? '#22c55e' : toastType === 'warning' ? '#f59e0b' : '#ef4444'};">
      {toastMsg}
    </div>
  {/if}
</div>

<style>
@keyframes fadein {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fadein { animation: fadein 0.3s; }
</style>