<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';
  import { goto } from '$app/navigation';

  let variables = [];
  let structures = [];
  let variablesImportedFiles = [];
  let variablesCount = 0;
  let structuresCount = 0;
  let projectInfo = null;

  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';
  let loading = false;
  let foundVariableFiles = [];
  let showStructureModal = false;
  let selectedStructure = null;
  let structureMembers = [];
  
  // Filtri e ricerca
  let searchQuery = '';
  let filterGroup = '';
  let filterType = '';
  let availableGroups = [];
  let availableTypes = [];
  
  // Card display settings
  let cardWidth = 180;
  let cardHeight = 170;
  let imageHeight = 90;
  let tableName = '';

  // Mappa dei tipi di variabili PremiumHMI
  const varTypeMap = {
    '0': 'Bool',
    '1': 'Byte',
    '2': 'SByte',
    '3': 'Int16',
    '4': 'UInt16',
    '5': 'Int32',
    '6': 'UInt32',
    '7': 'Int64',
    '8': 'Real',
    '9': 'String',
    '10': 'DateTime',
    '11': 'Struct',
    '12': 'Array'
  };

  // Mappa dei tipi di area
  const areaTypeMap = {
    '0': 'Input',
    '1': 'Output',
    '2': 'Flag',
    '3': 'Memory'
  };

  function getVarTypeName(code) {
    return varTypeMap[code] || code || 'Unknown';
  }

  function getAreaTypeName(code) {
    return areaTypeMap[code] || code || 'Unknown';
  }

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
      // Carica le informazioni del progetto
      projectInfo = await invoke('get_table_info', { tableName: tableName });
      
      // Carica i file di variabili già importati
      try {
        variablesImportedFiles = await invoke('get_variable_imported_files', { tableName: tableName });
      } catch (e) {
        variablesImportedFiles = [];
      }

      // Carica le variabili dal database
      try {
        variables = await invoke('get_variables', { tableName: tableName });
        variablesCount = variables.length;
        
        // Estrai gruppi e tipi unici per i filtri
        availableGroups = [...new Set(variables.map(v => v.var_group).filter(g => g))];
        availableTypes = [...new Set(variables.map(v => v.var_type).filter(t => t))];
      } catch (e) {
        variables = [];
        variablesCount = 0;
      }

      // Carica le strutture dal database
      try {
        structures = await invoke('get_structures', { tableName: tableName });
        structuresCount = structures.length;
      } catch (e) {
        structures = [];
        structuresCount = 0;
      }
      
      console.log('Loaded variables:', variables.length);
      console.log('Loaded structures:', structures.length);
      console.log('Project info:', projectInfo);

      // Cerca automaticamente i file .hmirealtimedb nella directory del progetto
      if (projectInfo && projectInfo.path) {
        try {
          foundVariableFiles = await invoke('get_variable_files_in_directory', { 
            directoryPath: projectInfo.path,
            tableName: tableName
          });
          
          if (foundVariableFiles.length > 0) {
            console.log('Found variable files automatically:', foundVariableFiles.length);
          }
        } catch (e) {
          console.log('No variable files found automatically:', e);
          foundVariableFiles = [];
        }
      }
    } catch (e) {
      console.error('Errore caricamento dati progetto:', e);
      toastMsg = $_('home.error_loading_project') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }

  async function importFoundFile(fileInfo) {
    loading = true;
    console.log('Importazione file variabili:', fileInfo);
    
    try {
      const result = await invoke('import_variable_file', {
        tableName: tableName,
        filePath: fileInfo.file_path
      });
      
      console.log('Risultato importazione:', result);
      
      toastMsg = result || `File ${fileInfo.file_name} importato con successo!`;
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
    return variablesImportedFiles.some(imported => 
      imported.file_path === fileInfo.file_path
    );
  }
  
  async function searchProjectFiles() {
    loading = true;
    
    try {
      // Prima prova a cercare automaticamente nella directory del progetto
      if (projectInfo && projectInfo.path) {
        console.log('Ricerca automatica nella directory:', projectInfo.path);
        foundVariableFiles = await invoke('get_variable_files_in_directory', { 
          directoryPath: projectInfo.path,
          tableName: tableName
        });
        
        if (foundVariableFiles.length > 0) {
          toastMsg = $_('database.variable_files_found', { values: { count: foundVariableFiles.length } });
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
          loading = false;
          return;
        }
      }
      
      // Se non trova file automaticamente, chiede di selezionarli manualmente
      toastMsg = $_('home.no_variable_files_found');
      toastType = 'info';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
      
      // Apre dialog per selezione multipla
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: false,
        multiple: true,
        filters: [{
          name: 'File di variabili',
          extensions: ['hmirealtimedb']
        }]
      });
      
      if (selected && Array.isArray(selected)) {
        const manualFiles = [];
        
        for (const filePath of selected) {
          const fileName = filePath.split(/[/\\]/).pop() || '';
          const fileNameLower = fileName.toLowerCase();
          
          if (fileNameLower.endsWith('.hmirealtimedb')) {
            manualFiles.push({
              file_name: fileName,
              file_path: filePath,
              file_type: 'hmirealtimedb'
            });
          }
        }
        
        foundVariableFiles = manualFiles;
        
        if (foundVariableFiles.length > 0) {
          toastMsg = `Selezionati ${foundVariableFiles.length} file di variabili!`;
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        } else {
          toastMsg = $_('home.no_valid_variable_files');
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

  async function showStructureDetails(structure) {
    selectedStructure = structure;
    loading = true;
    
    try {
      structureMembers = await invoke('get_structure_members', { 
        tableName: tableName,
        structureName: structure.name
      });
      showStructureModal = true;
    } catch (e) {
      console.error('Errore caricamento membri struttura:', e);
      toastMsg = 'Errore nel caricamento dei membri della struttura: ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }

  // Filtra le variabili in base ai criteri di ricerca
  $: filteredVariables = variables.filter(v => {
    const matchesSearch = !searchQuery || 
      v.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (v.description && v.description.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesGroup = !filterGroup || v.var_group === filterGroup;
    const matchesType = !filterType || v.var_type === filterType;
    return matchesSearch && matchesGroup && matchesType;
  });

  function openVariablesTableView() {
    goto(`/home/proj_man/variable_view/table_view?table=${encodeURIComponent(tableName)}`);
  }
</script>

<div class="min-h-screen flex flex-col" style="background: linear-gradient(135deg, #c9ffe7 0%, #e9e9ff 70%, #dcecff 100%);">
  
  <!-- TOAST NOTIFICATIONS -->
  {#if showToast}
    <div class="fixed bottom-8 right-8 z-50 px-6 py-3 rounded shadow-lg animate-fadein font-semibold text-white"
      style="background-color: {toastType === 'success' ? '#22c55e' : toastType === 'info' ? '#3b82f6' : '#ef4444'};">
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
        <p class="text-gray-700 text-sm">{$_('project.variables_subtitle')}</p>
      </div>
      
      <div class="flex gap-2 items-center">
        <button class="bg-purple-500 hover:bg-purple-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" on:click={searchProjectFiles} disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
          {$_('project.search_variables')}
        </button>
      </div>
    </div>
  </header>

  <!-- MAIN CONTENT -->
  <main class="flex-grow pt-5 px-5 mb-8" style="margin-top: 6rem; margin-bottom: 2rem;">
    <div class="w-full h-full overflow-y-auto pb-20" style="scrollbar-width: thin;">
      
      <!-- Sezione file variabili (trovati e importati unificati) -->
      {#if foundVariableFiles.length > 0 || variablesImportedFiles.length > 0}
        <div class="mb-6">
          <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
            <svg class="w-5 h-5 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
            {$_('database.variable_files_section')}
          </h3>
          <p class="text-sm text-gray-600 text-center mb-4">
            {$_('database.variable_import_description')}
          </p>
          <!-- Linea separatrice sopra -->
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mb-4"></div>
          <div class="flex flex-wrap justify-center gap-4">
            {#each foundVariableFiles as fileInfo}
              {@const alreadyImported = isFileAlreadyImported(fileInfo)}
              {@const importedData = variablesImportedFiles.find(f => f.file_path === fileInfo.file_path)}
              <div class="backdrop-blur-sm rounded-lg border p-2 text-center shadow-lg {alreadyImported ? 'bg-green-100/90 border-green-300/50' : 'bg-cyan-100/90 border-cyan-300/50'}"
                style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;">
                
                <div class="rounded-lg w-full mb-2 flex items-center justify-center relative {alreadyImported ? 'bg-green-200' : 'bg-cyan-200'}"
                  style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                  
                  {#if alreadyImported}
                    <svg class="w-12 h-12 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <div class="absolute top-1 right-1 bg-green-500 text-white text-xs px-1 py-0.5 rounded">
                      {importedData ? importedData.variables_count : ''} var
                    </div>
                  {:else}
                    <svg class="w-12 h-12 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
                    </svg>
                    <div class="absolute top-1 right-1 bg-cyan-500 text-white text-xs px-1 py-0.5 rounded">
                      {$_('database.not_imported_badge')}
                    </div>
                  {/if}
                </div>
                
                <h2 class="text-base font-semibold mb-1 truncate {alreadyImported ? 'text-green-800' : 'text-cyan-800'}">{$_('database.variables_file')}</h2>
                <p class="text-xs mb-2 truncate {alreadyImported ? 'text-green-600' : 'text-cyan-600'}" title={fileInfo.file_name}>{fileInfo.file_name}</p>
                
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
      
      <!-- Sezione Statistiche e Navigazione -->
      {#if variablesCount > 0 || structuresCount > 0}
        <div class="mb-6">
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mb-6"></div>
          
          <div class="flex flex-col xl:flex-row items-center justify-center gap-6">
            
            <!-- Card Variabili -->
            {#if variablesCount > 0}
              <div class="w-full max-w-sm">
                <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
                  <svg class="w-5 h-5 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
                  </svg>
                  {$_('database.variables_section')}
                </h3>
                <p class="text-sm text-gray-600 text-center mb-4">
                  {$_('database.variables_count', { values: { count: variablesCount } })}
                </p>
                
                <div class="flex justify-center">
                  <div class="bg-cyan-100/90 backdrop-blur-sm rounded-lg border border-cyan-300/50 p-2 text-center shadow-lg cursor-pointer hover:bg-cyan-200/90 transition-colors"
                    style="width: {cardWidth}px; min-width: {cardWidth}px; max-width: {cardWidth}px; height: {cardHeight}px;"
                    on:click={openVariablesTableView}
                    on:keypress={(e) => e.key === 'Enter' && openVariablesTableView()}
                    role="button"
                    tabindex="0">
                    
                    <div class="rounded-lg w-full bg-cyan-200 mb-2 flex items-center justify-center relative"
                      style="height: {imageHeight}px; min-height: {imageHeight}px; max-height: {imageHeight}px;">
                      <svg class="w-12 h-12 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4"></path>
                      </svg>
                      <div class="absolute top-1 right-1 bg-cyan-500 text-white text-xs px-1 py-0.5 rounded">
                        {variablesCount}
                      </div>
                    </div>
                    
                    <h2 class="text-base font-semibold mb-1 text-cyan-800">{$_('database.view_variables')}</h2>
                    <p class="text-xs text-cyan-600">{$_('database.click_to_open')}</p>
                  </div>
                </div>
              </div>
              
              <!-- Separatore verticale -->
              <div class="hidden xl:block w-px h-40 bg-gradient-to-b from-transparent via-black to-transparent"></div>
            {/if}
            
            <!-- Card Strutture -->
            {#if structuresCount > 0}
              <div class="w-full max-w-sm">
                <h3 class="text-lg font-semibold text-gray-800 mb-3 flex items-center justify-center gap-2">
                  <svg class="w-5 h-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
                  </svg>
                  {$_('database.structures_section')}
                </h3>
                <p class="text-sm text-gray-600 text-center mb-4">
                  {$_('database.structures_count', { values: { count: structuresCount } })}
                </p>
                
                <div class="flex flex-wrap justify-center gap-2">
                  {#each structures.slice(0, 6) as structure}
                    <button 
                      on:click={() => showStructureDetails(structure)}
                      class="bg-purple-100/90 backdrop-blur-sm rounded-lg border border-purple-300/50 p-2 text-center shadow-lg hover:bg-purple-200/90 transition-colors"
                      style="width: 120px;">
                      <p class="text-xs font-semibold text-purple-800 truncate" title={structure.name}>{structure.name}</p>
                    </button>
                  {/each}
                  {#if structures.length > 6}
                    <p class="text-xs text-gray-500 w-full text-center mt-2">+{structures.length - 6} {$_('database.more_structures')}</p>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
          
          <div class="w-full h-px bg-gradient-to-r from-transparent via-black to-transparent mt-6"></div>
        </div>
      {/if}
      
      
      
    </div>
  </main>
</div>

<!-- Modal dettagli struttura -->
{#if showStructureModal && selectedStructure}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg  max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden">
      <div class="p-4 border-b border-gray-200 flex justify-between items-center">
        <h3 class="text-lg font-semibold text-gray-900">{$_('database.structure_details')}: {selectedStructure.name}</h3>
        <button on:click={() => showStructureModal = false} class="text-gray-500 hover:text-gray-700">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      
      <div class="p-4 overflow-y-auto max-h-[60vh]">
        {#if selectedStructure.description}
          <p class="text-sm text-gray-600 mb-4">{selectedStructure.description}</p>
        {/if}
        
        <h4 class="font-semibold text-gray-800 mb-2">{$_('database.structure_members')}</h4>
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{$_('database.member_name')}</th>
              <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">{$_('database.member_type')}</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200">
            {#each structureMembers as member}
              <tr>
                <td class="px-4 py-2 text-sm text-gray-900">{member.member_name}</td>
                <td class="px-4 py-2 text-sm text-gray-600">
                  <span class="px-2 py-1 text-xs rounded-full bg-purple-100 text-purple-800">{getVarTypeName(member.member_type)}</span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      
      <div class="p-4 border-t border-gray-200 flex justify-end">
        <button on:click={() => showStructureModal = false} class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded">
          {$_('project.close')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .animate-fadein {
    animation: fadein 0.3s;
  }
  @keyframes fadein {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
