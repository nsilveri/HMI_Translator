<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';

  let languages = [];
  let tableName = '';
  let projectInfo = null;
  let foundTranslationFiles = [];
  let importedFiles = [];
  let recordsCount = 0;

  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';
  let showAddLanguageModal = false;
  let newLanguageCode = '';
  let newLanguageName = '';
  let loading = false;
  let showKeysModal = false;
  let foundKeys = [];
  let showProjectKeysModal = false;
  let projectKeys = [];
  let projectKeysDetails = [];
  let showExportModal = false;
  let exportPreview = null;
  let showAccentedCharsModal = false;
  let accentedCharacters = [];
  let selectedFixes = [];
  let selectedLanguages = [];
  let maxVisibleLanguages = 3;
  let showDeleteLanguageModal = false;
  let languageToDelete = null;
  
  // Computed property per le lingue da visualizzare
  $: visibleLanguages = selectedLanguages.length > 0 
    ? languages.filter(lang => selectedLanguages.includes(lang.code))
    : languages.slice(0, maxVisibleLanguages);
  
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
      
      // Carica i file già importati
      importedFiles = await invoke('get_imported_files', { projectName: tableName });
      
      // Carica il numero di record nel database
      const records = await invoke('get_records', { tableName: tableName });
      recordsCount = records.length;
      
      // Carica le chiavi del progetto dal database
      try {
        projectKeys = await invoke('get_project_keys', { projectName: tableName });
        console.log('Loaded project keys:', projectKeys.length);
      } catch (e) {
        console.error('Errore caricamento chiavi progetto:', e);
        projectKeys = [];
      }
      
      // Se abbiamo il percorso del progetto, scansiona i file di traduzione UNA SOLA VOLTA
      if (projectInfo && projectInfo.path) {
        console.log('Scanning directory for translation files:', projectInfo.path);
        try {
          foundTranslationFiles = await invoke('get_translation_files_in_directory', { 
            directoryPath: projectInfo.path,
            tableName: tableName
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
      
      console.log('Loaded languages:', languages);
      console.log('Project info:', projectInfo);
      console.log('Imported files:', importedFiles);
    } catch (e) {
      console.error('Errore caricamento dati progetto:', e);
      toastMsg = $_('home.error_loading_project') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }

  async function addLanguage() {
    if (!newLanguageCode.trim() || !newLanguageName.trim()) {
      toastMsg = $_('home.enter_language_code');
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2500);
      return;
    }

    loading = true;
    try {
      const result = await invoke('add_language_to_project', { 
        projectName: tableName, 
        languageCode: newLanguageCode.trim().toLowerCase(), 
        languageName: newLanguageName.trim() 
      });
      toastMsg = result;
      toastType = 'success';
      await loadProjectData(); // Ricarica i dati
      closeAddLanguageModal();
    } catch (e) {
      toastMsg = $_('home.error_adding_language') + ' ' + e;
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

  function toggleLanguageSelection(languageCode) {
    if (selectedLanguages.includes(languageCode)) {
      selectedLanguages = selectedLanguages.filter(code => code !== languageCode);
    } else if (selectedLanguages.length < maxVisibleLanguages) {
      selectedLanguages = [...selectedLanguages, languageCode];
    }
  }

  function isLanguageSelected(languageCode) {
    return selectedLanguages.includes(languageCode);
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
              languageCode = prompt($_('database.enter_language_code_prompt'));
              if (!languageCode) {
                toastMsg = $_('home.import_cancelled_language');
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
            toastMsg = $_('home.error_importing_file') + ' ' + error;
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
    console.log('Importazione file:', fileInfo);
    
    try {
      // Usa il backend per leggere e importare il file direttamente
      console.log('Chiamando import_translation_file_from_path con:', {
        tableName: tableName,
        languageCode: fileInfo.language_code,
        filePath: fileInfo.file_path
      });
      
      const result = await invoke('import_translation_file_from_path', {
        tableName: tableName,
        languageCode: fileInfo.language_code,
        filePath: fileInfo.file_path
      });
      
      console.log('Risultato importazione:', result);
      
      toastMsg = result || `File ${fileInfo.file_name} importato con successo per ${fileInfo.language_name}!`;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      // Ricarica i dati del progetto
      await loadProjectData();
      
    } catch (e) {
      console.error('Errore importazione file trovato:', e);
      toastMsg = $_('home.error_import_file') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  function isFileAlreadyImported(fileInfo) {
    return importedFiles.some(imported => 
      imported.file_path === fileInfo.file_path && 
      imported.language_code === fileInfo.language_code
    );
  }
  
  function openDatabaseViewer() {
    // Naviga alla pagina di visualizzazione del database
    window.location.href = `/home/table/view?table=${encodeURIComponent(tableName)}`;
  }

  async function exportTranslations() {
    if (!tableName) return;

    loading = true;
    try {
      // Get export preview first
      const preview = await invoke('get_export_preview', { tableName: tableName });
      
      // Show confirmation modal with preview info
      exportPreview = preview;
      showExportModal = true;
      
    } catch (e) {
      console.error('Errore nella preview esportazione:', e);
      toastMsg = $_('home.error_export_preview') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 4000);
    }
    loading = false;
  }

  async function confirmExport() {
    showExportModal = false;
    loading = true;
    
    try {
      const result = await invoke('export_translations_per_language', { tableName: tableName });
      toastMsg = result || 'Esportazione completata';
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 4000);
    } catch (e) {
      console.error('Errore esportazione lingue:', e);
      toastMsg = $_('home.error_exporting') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 4000);
    }
    loading = false;
  }

  function cancelExport() {
    showExportModal = false;
    exportPreview = null;
  }
  
  async function searchProjectFiles() {
    loading = true;
    
    try {
      // Prima prova a cercare automaticamente nella directory del progetto
      if (projectInfo && projectInfo.path) {
        console.log($_('database.automatic_search_directory'), projectInfo.path);
        foundTranslationFiles = await invoke('get_translation_files_in_directory', { 
          directoryPath: projectInfo.path,
          tableName: tableName
        });
        
        if (foundTranslationFiles.length > 0) {
          toastMsg = $_('database.translation_files_found', { values: { count: foundTranslationFiles.length } });
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
          loading = false;
          return;
        }
      }
      
      // Se non trova file automaticamente, chiede di selezionarli manualmente
      toastMsg = $_('home.no_translation_files_found');
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
          toastMsg = $_('home.no_valid_translation_files');
          toastType = 'warning';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        }
      }
      
    } catch (e) {
      console.error('Errore nella ricerca dei file:', e);
      toastMsg = $_('home.error_searching_files') + ' ' + e;
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
      toastMsg = $_('home.project_path_unavailable');
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
      
      toastMsg = $_('database.keys_found', { values: { count: foundKeys.length } });
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
    } catch (e) {
      console.error('Errore nella ricerca delle chiavi:', e);
      toastMsg = $_('home.error_searching_keys') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  async function importFoundKeys() {
    if (foundKeys.length === 0) {
      toastMsg = $_('home.no_keys_to_import');
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
      toastMsg = $_('home.error_importing_keys') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }
  
  async function showProjectKeys() {
    loading = true;
    
    try {
      projectKeysDetails = await invoke('get_project_keys_with_status', { 
        projectName: tableName 
      });
      showProjectKeysModal = true;
    } catch (e) {
      console.error('Errore caricamento dettagli chiavi:', e);
      toastMsg = $_('home.error_loading_key_details') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }

  async function checkAccentedCharacters() {
    loading = true;
    
    try {
      accentedCharacters = await invoke('check_accented_characters', { 
        tableName: tableName 
      });
      selectedFixes = accentedCharacters.map(char => ({
        id: char.id,
        column: char.column,
        newValue: char.suggested_value,
        selected: true
      }));
      showAccentedCharsModal = true;
    } catch (e) {
      console.error('Errore controllo caratteri accentati:', e);
      toastMsg = $_('home.error_checking_accents') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }

  async function fixAccentedCharacters() {
    const fixesToApply = selectedFixes.filter(fix => fix.selected);
    
    if (fixesToApply.length === 0) {
      toastMsg = $_('home.no_correction_selected');
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      return;
    }

    loading = true;
    
    try {
      const result = await invoke('fix_accented_characters', { 
        tableName: tableName,
        fixes: fixesToApply
      });
      
      toastMsg = result;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      showAccentedCharsModal = false;
      
    } catch (e) {
      console.error('Errore correzione caratteri accentati:', e);
      toastMsg = $_('home.error_fixing_accents') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }

  function toggleFix(index) {
    selectedFixes[index].selected = !selectedFixes[index].selected;
  }

  function confirmDeleteLanguage(languageCode, languageName) {
    languageToDelete = { code: languageCode, name: languageName };
    showDeleteLanguageModal = true;
  }

  async function deleteLanguage() {
    if (!languageToDelete) return;

    loading = true;
    try {
      console.log('Tentativo eliminazione lingua:', {
        projectName: tableName,
        languageCode: languageToDelete.code,
        languageName: languageToDelete.name
      });
      
      const result = await invoke('remove_language_from_project', { 
        projectName: tableName, 
        languageCode: languageToDelete.code 
      });
      
      toastMsg = result || `Lingua ${languageToDelete.name} eliminata con successo`;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      // Ricarica i dati del progetto
      await loadProjectData();
      
      // Chiudi il modal
      showDeleteLanguageModal = false;
      languageToDelete = null;
    } catch (e) {
      console.error('Errore eliminazione lingua:', e);
      console.log('Lingue attualmente caricate:', languages);
      toastMsg = $_('home.error_deleting_language') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    loading = false;
  }

  function cancelDeleteLanguage() {
    showDeleteLanguageModal = false;
    languageToDelete = null;
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
        ← {$_('project.back')}
      </button>
      
      <div class="text-center flex-1">
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('project.title')} {tableName}</h1>
        <p class="text-gray-700 text-sm">{$_('project.languages_subtitle')}</p>
      </div>
      
      <div class="flex gap-2 items-center">
        <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={importTranslationFile} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"></path>
          </svg>
          {loading ? $_('project.importing') : $_('project.import_file')}
        </button>
        <button class="bg-purple-500 hover:bg-purple-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={searchProjectFiles} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
          {$_('project.search_project')}
        </button>
        <button class="bg-amber-500 hover:bg-amber-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={exportTranslations} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v14m7-7H5"></path>
          </svg>
          {$_('project.export_languages')}
        </button>
        <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={findProjectKeys}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
          {$_('project.find_keys')}
        </button>
        <button class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={checkAccentedCharacters} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.771-.833-2.693-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
          </svg>
          {$_('project.character_check')}
        </button>
        <button class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={openAddLanguageModal}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
          </svg>
          {$_('project.add_language')}
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
            {$_('database.translation_files_available')}
          </h3>
          <p class="text-sm text-gray-600 text-center mb-4">
            {$_('database.import_description')}
          </p>
          <!-- Linea separatrice sopra -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mb-4"></div>
          <div class="flex flex-wrap justify-center gap-4">
            {#each foundTranslationFiles as fileInfo}
              {@const alreadyImported = isFileAlreadyImported(fileInfo)}
              <div class="backdrop-blur-sm rounded-lg border p-2 text-center shadow-lg {alreadyImported ? 'bg-green-100/90 border-green-300/50' : 'bg-yellow-100/90 border-yellow-300/50'}"
                style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                
                <div class="rounded-lg w-full mb-2 flex items-center justify-center relative {alreadyImported ? 'bg-green-200' : 'bg-yellow-200'}"
                  style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                  
                  {#if alreadyImported}
                    <!-- Icona di checkmark per file già importato -->
                    <svg class="w-12 h-12 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <!-- Badge di stato -->
                    <div class="absolute top-1 right-1 bg-green-500 text-white text-xs px-1 py-0.5 rounded">
                      {$_('database.imported_badge')}
                    </div>
                  {:else}
                    <!-- Icona di file normale -->
                    <svg class="w-12 h-12 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                    </svg>
                  {/if}
                </div>
                
                <h2 class="text-base font-semibold mb-1 truncate {alreadyImported ? 'text-green-800' : 'text-yellow-800'}">{fileInfo.language_name}</h2>
                <p class="text-xs mb-2 truncate {alreadyImported ? 'text-green-600' : 'text-yellow-600'}" title={fileInfo.file_name}>{fileInfo.file_name}</p>
                
                <div class="flex justify-center">
                  {#if alreadyImported}
                    <button 
                      on:click={() => importFoundFile(fileInfo)}
                      disabled={loading}
                      class="bg-blue-500 hover:bg-blue-700 disabled:bg-gray-400 text-white font-bold py-1 px-3 rounded text-xs flex items-center gap-1"
                      title="{$_('database.reimport_tooltip')}">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                      </svg>
                      {$_('database.reimport_button')}
                    </button>
                  {:else}
                    <button 
                      on:click={() => importFoundFile(fileInfo)}
                      disabled={loading}
                      class="bg-green-500 hover:bg-green-700 disabled:bg-gray-400 text-white font-bold py-1 px-3 rounded text-xs flex items-center gap-1"
                      title="{$_('database.import_tooltip')}">
                      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"></path>
                      </svg>
                      {$_('table.import_file_button')}
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
          <!-- Linea separatrice sotto -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mt-4"></div>
        </div>
      {/if}
      
      <!-- Sezioni principali del progetto in griglia responsive -->
      {#if recordsCount > 0 || languages.length > 0 || projectKeys.length > 0}
        <div class="mb-6">
          <!-- Linea separatrice sopra -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mb-6"></div>
          
          <!-- Layout responsive con separatori verticali -->
          <div class="flex flex-col xl:flex-row items-center justify-center gap-6">
            
            <!-- Sezione Database del progetto -->
            {#if recordsCount > 0}
              <div class="w-full max-w-sm">
                <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
                  <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 1.79 4 4 4h8c0 2.21 1.79 4 4 4h8c0-2.21-1.79-4-4-4V7c0-2.21-1.79-4-4-4H8c-2.21 0-4 1.79-4 4z"></path>
                  </svg>
                  {$_('project.database_section')}
                </h3>
                <p class="text-sm text-gray-600 text-center mb-4">
                  {$_('project.database_description', { values: { count: recordsCount } })}
                </p>
                
                <div class="flex justify-center">
                  <div class="bg-blue-100/90 backdrop-blur-sm rounded-lg border border-blue-300/50 p-2 text-center shadow-lg"
                    style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                    
                    <div class="rounded-lg w-full bg-blue-200 mb-2 flex items-center justify-center relative"
                      style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                      
                      <!-- Icona database -->
                      <svg class="w-12 h-12 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 1.79 4 4 4h8c0 2.21 1.79 4 4 4h8c0-2.21-1.79-4-4-4V7c0-2.21-1.79-4-4-4H8c-2.21 0-4 1.79-4 4z"></path>
                      </svg>
                      
                      <!-- Badge con numero record -->
                      <div class="absolute top-1 right-1 bg-blue-500 text-white text-xs px-1 py-0.5 rounded">
                        {recordsCount} record
                      </div>
                    </div>
                    
                    <h2 class="text-base font-semibold text-blue-800 mb-1 truncate">{$_('project.database_card_title')}</h2>
                    <p class="text-xs text-blue-600 mb-2 truncate">{$_('project.database_card_subtitle')}</p>
                    
                    <div class="flex justify-center">
                      <button 
                        on:click={openDatabaseViewer}
                        disabled={loading}
                        class="bg-blue-500 hover:bg-blue-700 disabled:bg-gray-400 text-white font-bold py-1 px-3 rounded text-xs flex items-center gap-1"
                        title="{$_('home.open_database_viewer')}">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                        </svg>
                        {$_('project.view_db')}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
            
            <!-- Separatore 1: orizzontale su mobile, verticale su desktop -->
            {#if recordsCount > 0 && (languages.length > 0 || projectKeys.length > 0)}
              <!-- Linea orizzontale su schermi piccoli -->
              <div class="xl:hidden w-full h-px bg-gradient-to-r from-transparent via-black to-transparent my-4"></div>
              <!-- Linea verticale su schermi grandi -->
              <div class="hidden xl:block w-px h-64 bg-gradient-to-b from-transparent via-black to-transparent"></div>
            {/if}
            
            <!-- Sezione Lingue configurate -->
            {#if languages.length > 0}
              <div class="w-full max-w-sm">
                <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
                  <svg class="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                  </svg>
                  {$_('project.languages_section')}
                </h3>
                <p class="text-sm text-gray-600 text-center mb-4">
                  {$_('project.languages_description', { values: { count: languages.length, plural: languages.length === 1 ? $_('project.language_configured') : $_('project.languages_configured') } })}
                </p>               
                
                <!-- Container scorribile: mostra 3 lingue alla volta -->
                <div class="overflow-y-auto" style="scrollbar-width: thin; height: 192px; max-height: 192px;">
                  <div class="flex flex-col gap-2 pr-1">
                    {#each languages as language}
                      <div class="bg-green-100/90 backdrop-blur-sm rounded-lg border border-green-300/50 p-2 text-center shadow-lg flex items-center gap-3 group hover:bg-green-200/90 transition-colors" style="min-height: 56px;">
                        <div class="w-8 h-8 bg-green-200 rounded flex items-center justify-center flex-shrink-0">
                          <svg class="w-4 h-4 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                          </svg>
                        </div>
                        <div class="flex-1 min-w-0">
                          <h4 class="text-sm font-semibold text-green-800 truncate">{language.name}</h4>
                          <p class="text-xs text-green-600">({language.code})</p>
                        </div>
                        <!-- Pulsante Elimina - visibile al hover -->
                        <div class="flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                          <button
                            on:click={() => confirmDeleteLanguage(language.code, language.name)}
                            disabled={loading}
                            class="bg-red-500 hover:bg-red-600 disabled:bg-gray-400 text-white p-1 rounded text-xs flex items-center gap-1 transition-colors"
                            title="Elimina lingua {language.name}">
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                            </svg>
                            Elimina
                          </button>
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              </div>
            {/if}
            
            <!-- Separatore 2: orizzontale su mobile, verticale su desktop -->
            {#if languages.length > 0 && projectKeys.length > 0}
              <!-- Linea orizzontale su schermi piccoli -->
              <div class="xl:hidden w-full h-px bg-gradient-to-r from-transparent via-black to-transparent my-4"></div>
              <!-- Linea verticale su schermi grandi -->
              <div class="hidden xl:block w-px h-64 bg-gradient-to-b from-transparent via-black to-transparent"></div>
            {/if}
            
            <!-- Sezione Chiavi del progetto -->
            {#if projectKeys.length > 0}
              <div class="w-full max-w-sm">
                <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
                  <svg class="w-5 h-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1721 9z"></path>
                  </svg>
                  {$_('project.keys_section')}
                </h3>
                <p class="text-sm text-gray-600 text-center mb-4">
                  {$_('project.keys_description', { values: { count: projectKeys.length, plural: projectKeys.length === 1 ? $_('project.key_found') : $_('project.keys_found') } })}
                </p>
                
                <div class="flex justify-center">
                  <div class="bg-purple-100/90 backdrop-blur-sm rounded-lg border border-purple-300/50 p-2 text-center shadow-lg"
                    style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                    
                    <div class="rounded-lg w-full bg-purple-200 mb-2 flex items-center justify-center relative"
                      style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                      <svg class="w-12 h-12 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1721 9z"></path>
                      </svg>
                      
                      <!-- Badge con numero chiavi -->
                      <div class="absolute top-1 right-1 bg-purple-500 text-white text-xs px-1 py-0.5 rounded">
                        {projectKeys.length} {$_('project.keys.badge_keys')}
                      </div>
                    </div>
                    
                    <h2 class="text-base font-semibold text-purple-800 mb-1 truncate">{$_('project.keys.title')}</h2>
                    <p class="text-xs text-purple-600 mb-2 truncate">{$_('project.keys.found', { values: { count: projectKeys.length } })}</p>
                    
                    <div class="flex justify-center">
                      <button 
                        on:click={showProjectKeys}
                        class="bg-purple-500 hover:bg-purple-700 text-white font-bold py-1 px-3 rounded text-xs flex items-center gap-1">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                        </svg>
                        {$_('project.keys.view_button')}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
          
          <!-- Linea separatrice sotto -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mt-6"></div>
        </div>
      {/if}

      {#if languages.length === 0 && foundTranslationFiles.length === 0 && projectKeys.length === 0}
        <div class="text-center text-gray-500 py-8">
          <p class="mb-4">{$_('database.no_languages_configured')}</p>
          <button class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded" on:click={openAddLanguageModal}>
            {$_('database.add_first_language')}
          </button>
        </div>
      {/if}
    </div>
  </main>

  <!-- ADD LANGUAGE MODAL -->
  {#if showAddLanguageModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div class="bg-white rounded-lg shadow-lg p-6 w-full max-w-sm">
        <h2 class="text-lg font-semibold mb-4">{$_('project.add_language_modal_title')}</h2>
        <div class="mb-4">
          <label for="languageCode" class="block text-sm font-medium text-gray-700 mb-2">{$_('project.add_language_code')}</label>
          <input 
            id="languageCode"
            type="text" 
            bind:value={newLanguageCode} 
            placeholder="{$_('project.add_language_code_placeholder')}" 
            class="w-full border rounded px-3 py-2 text-sm"
            maxlength="5"
          />
        </div>
        <div class="mb-4">
          <label for="languageName" class="block text-sm font-medium text-gray-700 mb-2">{$_('project.add_language_name')}</label>
          <input 
            id="languageName"
            type="text" 
            bind:value={newLanguageName} 
            placeholder="{$_('project.add_language_name_placeholder')}" 
            class="w-full border rounded px-3 py-2 text-sm"
          />
        </div>
        <div class="flex justify-end gap-3">
          <button 
            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded" 
            on:click={closeAddLanguageModal}
            disabled={loading}
          >
            {$_('project.add_language_cancel')}
          </button>
          <button 
            class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded" 
            on:click={addLanguage}
            disabled={loading}
          >
            {loading ? $_('project.adding') : $_('project.add_language_confirm')}
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
            {$_('project.keys_modal_title')} ({foundKeys.length})
          </h2>
          <button 
            class="text-gray-500 hover:text-gray-700"
            on:click={() => showKeysModal = false}
            aria-label="{$_('project.keys_modal_close')}"
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto">
          {#if foundKeys.length === 0}
            <p class="text-gray-500 text-center py-8">{$_('project.keys_modal_no_keys')}</p>
          {:else}
            {@const groupedKeys = foundKeys.reduce((groups, keyInfo) => {
              const fileName = keyInfo.file;
              if (!groups[fileName]) {
                groups[fileName] = [];
              }
              groups[fileName].push(keyInfo);
              return groups;
            }, {})}
            
            <div class="space-y-4">
              {#each Object.entries(groupedKeys) as [fileName, fileKeys]}
                <div class="bg-white rounded-lg border border-gray-300 shadow-sm overflow-hidden">
                  <!-- Header del file -->
                  <div class="bg-blue-50 px-4 py-3 border-b border-blue-200">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center gap-2">
                        <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                        </svg>
                        <span class="text-lg font-semibold text-blue-800">{fileName}</span>
                      </div>
                      <div class="text-sm text-blue-600 bg-blue-100 px-3 py-1 rounded-full">
                        {fileKeys.length} {$_('project.keys_modal_files_count', { values: { count: fileKeys.length } })}
                      </div>
                    </div>
                  </div>
                  
                  <!-- Lista delle chiavi del file -->
                  <div class="px-4 py-3 max-h-64 overflow-y-auto">
                    <div class="space-y-2">
                      {#each fileKeys as keyInfo, index}
                        <div class="p-3 bg-gray-50 rounded border border-gray-200">
                          <div class="flex items-start justify-between mb-2">
                            <div class="text-xs font-semibold text-gray-500 uppercase tracking-wide">
                              {$_('project.keys_modal_key_number', { values: { number: index + 1 } })}
                            </div>
                          </div>
                          
                          <!-- Chiave estratta -->
                          <div class="mb-2">
                            <div class="text-xs font-medium text-gray-600 mb-1">{$_('project.keys_modal_extracted_key')}:</div>
                            <div class="p-2 bg-green-50 rounded border border-green-200">
                              <span class="font-mono text-sm text-green-800 break-words">{keyInfo.key}</span>
                            </div>
                          </div>
                          
                          <!-- Riga completa (troncata) -->
                          <div>
                            <div class="text-xs font-medium text-gray-600 mb-1">{$_('project.keys_modal_file_line')}:</div>
                            <div class="p-2 bg-gray-100 rounded border border-gray-200 max-h-16 overflow-y-auto">
                              <span class="font-mono text-xs text-gray-700 break-all">{keyInfo.full_line}</span>
                            </div>
                          </div>
                        </div>
                      {/each}
                    </div>
                  </div>
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
            {$_('project.keys_modal_close')}
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
              {loading ? $_('project.keys_modal_importing') : $_('project.keys_modal_import_all_keys', { values: { count: foundKeys.length } })}
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- PROJECT KEYS MODAL -->
  {#if showProjectKeysModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div class="bg-white rounded-lg shadow-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-lg font-semibold flex items-center gap-2">
            <svg class="w-5 h-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"></path>
            </svg>
            {$_('project.project_keys_modal_title')} ({projectKeysDetails.length})
          </h2>
          <button 
            class="text-gray-500 hover:text-gray-700"
            on:click={() => showProjectKeysModal = false}
            aria-label="{$_('project.project_keys_modal_close')}"
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
        
        <div class="flex-1 overflow-y-auto">
          {#if projectKeysDetails.length === 0}
            <p class="text-gray-500 text-center py-8">{$_('project.project_keys_modal_no_keys')}</p>
          {:else}
            <!-- Statistiche generali -->
            {@const totalKeys = projectKeysDetails.length}
            {@const translatedKeys = projectKeysDetails.filter(k => k.exists_in_translations).length}
            {@const missingTranslations = totalKeys - translatedKeys}
            
            <div class="mb-4 p-3 bg-purple-50 rounded-lg border border-purple-200">
              <div class="flex items-center justify-center gap-6 text-sm">
                <div class="flex items-center gap-2">
                  <div class="w-3 h-3 bg-green-500 rounded-full"></div>
                  <span class="text-green-700 font-medium">{translatedKeys} {$_('project.project_keys_modal_translated')}</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-3 h-3 bg-orange-500 rounded-full"></div>
                  <span class="text-orange-700 font-medium">{missingTranslations} {$_('project.project_keys_modal_missing')}</span>
                </div>
              </div>
            </div>
            
            <div class="space-y-3">
              {#each projectKeysDetails as keyDetail, index}
                <div class="bg-white rounded-lg px-4 py-3 border border-gray-300 shadow-sm">
                  <!-- Header della chiave -->
                  <div class="flex items-center justify-between mb-2 pb-2 border-b border-gray-200">
                    <div class="flex items-center gap-2">
                      <svg class="w-4 h-4 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1721 9z"></path>
                      </svg>
                      <span class="text-sm font-medium text-purple-800">{$_('project.project_keys_modal_key_number', { values: { number: index + 1 } })}</span>
                    </div>
                    
                    <!-- Status della traduzione -->
                    <div class="flex items-center gap-2">
                      {#if keyDetail.exists_in_translations}
                        <div class="flex items-center gap-1 text-green-600 bg-green-100 px-2 py-1 rounded-full">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                          </svg>
                          <span class="text-xs font-medium">{$_('project.project_keys_modal_present_label')}</span>
                        </div>
                      {:else}
                        <div class="flex items-center gap-1 text-orange-600 bg-orange-100 px-2 py-1 rounded-full">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.966-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
                          </svg>
                          <span class="text-xs font-medium">{$_('project.project_keys_modal_missing_label')}</span>
                        </div>
                      {/if}
                    </div>
                  </div>
                  
                  <!-- Contenuto della chiave -->
                  <div>
                    <div class="text-xs font-medium text-gray-600 mb-1">{$_('project.project_keys_modal_project_key')}:</div>
                    <div class="p-2 bg-purple-50 rounded border border-purple-200">
                      <span class="font-mono text-sm text-purple-800 break-words">{keyDetail.key}</span>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
        
        <div class="flex justify-end gap-3 mt-4 pt-4 border-t">
          <button 
            class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded" 
            on:click={() => showProjectKeysModal = false}
          >
            {$_('project.project_keys_modal_close')}
          </button>
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

  <!-- Modal di conferma esportazione -->
  {#if showExportModal && exportPreview}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            <svg class="w-6 h-6 text-blue-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3M3 17V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path>
            </svg>
            <h3 class="text-lg font-semibold text-gray-900">{$_('database.confirm_export_title')}</h3>
          </div>
          
          <div class="mb-6">
            <p class="text-gray-600 mb-4">
              <strong>{$_('database.export_project_label')}</strong> {exportPreview.projectName}<br>
              <strong>{$_('database.export_path_label')}</strong> {exportPreview.projectPath}<br>
              <strong>{$_('database.export_languages_label')}</strong> {exportPreview.languageCount}
            </p>

            <div class="mb-4">
              <h4 class="font-semibold text-gray-800 mb-2">{$_('database.export_files_created')}</h4>
              <div class="bg-green-50 rounded-lg p-3 max-h-32 overflow-y-auto">
                {#each exportPreview.exportFiles as file}
                  <div class="flex items-center text-green-700 text-sm mb-1">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                    {file}
                  </div>
                {/each}
              </div>
            </div>

            {#if exportPreview.backupFiles.length > 0}
              <div class="mb-4">
                <h4 class="font-semibold text-gray-800 mb-2">{$_('database.export_files_backed_up')}</h4>
                <div class="bg-yellow-50 rounded-lg p-3 max-h-32 overflow-y-auto">
                  {#each exportPreview.backupFiles as file}
                    <div class="flex items-center text-yellow-700 text-sm mb-1">
                      <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3M3 17V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path>
                      </svg>
                      {file}
                    </div>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="mb-4">
                <div class="bg-gray-50 rounded-lg p-3">
                  <div class="flex items-center text-gray-600 text-sm">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    {$_('database.export_no_backup_files')}
                  </div>
                </div>
              </div>
            {/if}
          </div>

          <div class="flex justify-end gap-3">
            <button
              on:click={cancelExport}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
              {$_('database.export_cancel')}
            </button>
            <button
              on:click={confirmExport}
              class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3M3 17V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path>
              </svg>
              {$_('database.export_confirm')}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal controllo caratteri accentati -->
  {#if showAccentedCharsModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg shadow-xl max-w-6xl w-full mx-4 max-h-[90vh] overflow-hidden">
        <div class="p-6">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center">
              <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.771-.833-2.693-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
              </svg>
              <h3 class="text-lg font-semibold text-gray-900">{$_('project.accented_chars_modal_title')}</h3>
            </div>
            <button on:click={() => showAccentedCharsModal = false} class="text-gray-400 hover:text-gray-600">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          
          {#if accentedCharacters.length === 0}
            <div class="text-center py-8">
              <div class="bg-green-50 rounded-lg p-6">
                <svg class="w-16 h-16 text-green-500 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
                <h4 class="text-lg font-semibold text-green-800 mb-2">{$_('project.accented_chars_no_found')}</h4>
                <p class="text-green-600">{$_('project.accented_chars_no_found_desc')}</p>
              </div>
            </div>
          {:else}
            <div class="mb-4">
              <p class="text-gray-600 mb-4">
                {$_('project.accented_chars_found', { values: { count: accentedCharacters.length } })}
                {$_('project.accented_chars_select')}
              </p>

              <div class="max-h-96 overflow-y-auto border rounded-lg">
                <table class="w-full text-sm">
                  <thead class="bg-gray-50 sticky top-0">
                    <tr>
                      <th class="px-3 py-2 text-left">
                        <input type="checkbox" 
                               checked={selectedFixes.every(fix => fix.selected)}
                               on:change={(e) => {
                                 const checked = e.target.checked;
                                 selectedFixes = selectedFixes.map(fix => ({ ...fix, selected: checked }));
                               }}
                               class="rounded">
                      </th>
                      <th class="px-3 py-2 text-left">{$_('project.accented_chars_row')}</th>
                      <th class="px-3 py-2 text-left">{$_('project.accented_chars_key')}</th>
                      <th class="px-3 py-2 text-left">{$_('project.accented_chars_column')}</th>
                      <th class="px-3 py-2 text-left">{$_('project.accented_chars_original')}</th>
                      <th class="px-3 py-2 text-left">{$_('project.accented_chars_suggested')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each accentedCharacters as char, index}
                      <tr class="border-t hover:bg-gray-50">
                        <td class="px-3 py-2">
                          <input type="checkbox" 
                                 bind:checked={selectedFixes[index].selected}
                                 class="rounded">
                        </td>
                        <td class="px-3 py-2 text-gray-600">#{char.row_number}</td>
                        <td class="px-3 py-2 font-mono text-sm">{char.key || $_('project.key_empty')}</td>
                        <td class="px-3 py-2">
                          <span class="bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs font-semibold">
                            {char.column}
                          </span>
                        </td>
                        <td class="px-3 py-2 max-w-xs">
                          <div class="bg-red-50 border border-red-200 rounded p-2 text-sm">
                            {char.original_value}
                          </div>
                        </td>
                        <td class="px-3 py-2 max-w-xs">
                          <div class="bg-green-50 border border-green-200 rounded p-2 text-sm">
                            {char.suggested_value}
                          </div>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>

            <div class="flex justify-end gap-3">
              <button
                on:click={() => showAccentedCharsModal = false}
                class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
                {$_('project.accented_chars_cancel')}
              </button>
              <button
                on:click={fixAccentedCharacters}
                disabled={selectedFixes.filter(fix => fix.selected).length === 0 || loading}
                class="bg-red-500 hover:bg-red-600 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                </svg>
                {loading ? $_('project.accented_chars_fixing') : $_('project.accented_chars_fix', { values: { count: selectedFixes.filter(fix => fix.selected).length } })}
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal di conferma eliminazione lingua -->
  {#if showDeleteLanguageModal && languageToDelete}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z"></path>
            </svg>
            <h3 class="text-lg font-semibold text-gray-900">{$_('project.delete_language_modal_title')}</h3>
          </div>
          <p class="text-gray-600 mb-6">
            {@html $_('project.delete_language_modal_message', { values: { name: languageToDelete.name, code: languageToDelete.code } })}
            <br><br>
            <span class="text-red-600 font-medium">{$_('project.delete_language_modal_warning')}</span>
          </p>
          <div class="flex justify-end gap-3">
            <button
              on:click={cancelDeleteLanguage}
              disabled={loading}
              class="bg-gray-500 hover:bg-gray-600 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded">
              {$_('project.delete_language_modal_cancel')}
            </button>
            <button
              on:click={deleteLanguage}
              disabled={loading}
              class="bg-red-500 hover:bg-red-600 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
              {loading ? $_('project.delete_language_modal_deleting') : $_('project.delete_language_modal_confirm')}
            </button>
          </div>
        </div>
      </div>
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