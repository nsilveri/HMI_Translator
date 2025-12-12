<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';

  let tableName = '';
  let variables = [];
  let structures = [];
  let loading = false;
  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';
  let searchTerm = '';
  let filteredVariables = [];
  
  // Filtri
  let filterGroup = '';
  let filterType = '';
  let filterAreaType = '';
  let availableGroups = [];
  let availableTypes = [];
  let availableAreaTypes = [];
  
  // Modal struttura
  let showStructureModal = false;
  let selectedStructure = null;
  let structureMembers = [];
  
  // Modal lista strutture
  let showStructuresListModal = false;
  
  // Modal dettagli variabile
  let showVariableModal = false;
  let selectedVariable = null;
  
  // Ordinamento
  let sortColumn = 'name';
  let sortDirection = 'asc';

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

  // Colori per i tipi
  const typeColors = {
    'Bool': 'bg-green-100 text-green-800',
    'Byte': 'bg-blue-100 text-blue-800',
    'SByte': 'bg-blue-100 text-blue-800',
    'Int16': 'bg-indigo-100 text-indigo-800',
    'UInt16': 'bg-indigo-100 text-indigo-800',
    'Int32': 'bg-purple-100 text-purple-800',
    'UInt32': 'bg-purple-100 text-purple-800',
    'Int64': 'bg-pink-100 text-pink-800',
    'Real': 'bg-orange-100 text-orange-800',
    'String': 'bg-yellow-100 text-yellow-800',
    'DateTime': 'bg-teal-100 text-teal-800',
    'Struct': 'bg-red-100 text-red-800',
    'Array': 'bg-cyan-100 text-cyan-800'
  };

  function getVarTypeName(code) {
    return varTypeMap[code] || code || 'Unknown';
  }

  function getAreaTypeName(code) {
    return areaTypeMap[code] || code || 'Unknown';
  }

  function getTypeColor(code) {
    const typeName = getVarTypeName(code);
    return typeColors[typeName] || 'bg-gray-100 text-gray-800';
  }

  onMount(async () => {
    const urlParams = $page.url.searchParams;
    tableName = urlParams.get('table') || '';
    console.log('Table name:', tableName);

    if (!tableName) {
      console.log('No table name in URL');
      return;
    }

    await loadDatabaseData();
  });

  async function loadDatabaseData() {
    loading = true;
    try {
      // Carica le variabili
      variables = await invoke('get_variables', { tableName: tableName });
      
      // Carica le strutture
      structures = await invoke('get_structures', { tableName: tableName });
      
      // Estrai gruppi, tipi e aree uniche per i filtri
      availableGroups = [...new Set(variables.map(v => v.var_group).filter(g => g && g.trim()))].sort();
      availableTypes = [...new Set(variables.map(v => v.var_type).filter(t => t !== null && t !== undefined))].sort((a, b) => parseInt(a) - parseInt(b));
      availableAreaTypes = [...new Set(variables.map(v => v.area_type).filter(a => a !== null && a !== undefined))].sort();
      
      console.log('Loaded variables:', variables.length);
      console.log('Loaded structures:', structures.length);
      console.log('Available groups:', availableGroups);
      console.log('Available types:', availableTypes);
      
    } catch (e) {
      console.error('Errore nel caricamento dei dati:', e);
      toastMsg = $_('database.error_loading_data') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    loading = false;
  }

  // Filtra le variabili
  function filterVariables(vars, search, group, type, areaType) {
    let filtered = vars;
    
    // Filtro per gruppo
    if (group && group !== '') {
      filtered = filtered.filter(v => v.var_group === group);
    }
    
    // Filtro per tipo
    if (type && type !== '') {
      filtered = filtered.filter(v => v.var_type === type);
    }
    
    // Filtro per area type
    if (areaType && areaType !== '') {
      filtered = filtered.filter(v => v.area_type === areaType);
    }
    
    // Filtro per ricerca
    if (search && search.trim()) {
      const term = search.toLowerCase();
      filtered = filtered.filter(v => {
        return (v.name && v.name.toLowerCase().includes(term)) ||
               (v.description && v.description.toLowerCase().includes(term)) ||
               (v.var_group && v.var_group.toLowerCase().includes(term)) ||
               (v.dynamic_settings && v.dynamic_settings.toLowerCase().includes(term));
      });
    }
    
    return filtered;
  }

  // Ordina le variabili
  function sortVariables(vars, column, direction) {
    return [...vars].sort((a, b) => {
      let valA = a[column] || '';
      let valB = b[column] || '';
      
      // Converti in stringa per confronto
      if (typeof valA !== 'string') valA = String(valA);
      if (typeof valB !== 'string') valB = String(valB);
      
      const comparison = valA.localeCompare(valB, undefined, { numeric: true, sensitivity: 'base' });
      return direction === 'asc' ? comparison : -comparison;
    });
  }

  function toggleSort(column) {
    if (sortColumn === column) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = column;
      sortDirection = 'asc';
    }
  }

  // Estrai l'indirizzo dalla stringa dynamic_settings
  function extractAddress(dynamicSettings) {
    if (!dynamicSettings) return '-';
    const match = dynamicSettings.match(/Addr=([^|]+)/);
    return match ? match[1] : '-';
  }

  // Estrai Device dalla stringa dynamic_settings
  function extractDevice(dynamicSettings) {
    if (!dynamicSettings) return '-';
    const match = dynamicSettings.match(/Device=([^|]+)/);
    return match ? match[1] : '-';
  }

  // Mostra dettagli struttura
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
      toastMsg = $_('variables.error_loading') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    
    loading = false;
  }

  // Mostra dettagli variabile
  function showVariableDetails(variable) {
    selectedVariable = variable;
    showVariableModal = true;
  }

  // Reattività
  $: filteredVariables = sortVariables(
    filterVariables(variables, searchTerm, filterGroup, filterType, filterAreaType),
    sortColumn,
    sortDirection
  );

  function clearFilters() {
    searchTerm = '';
    filterGroup = '';
    filterType = '';
    filterAreaType = '';
  }

  function goBack() {
    window.history.back();
  }

  // Esporta in CSV
  async function exportToCSV() {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const filePath = await save({
        defaultPath: `${tableName}_variables.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }]
      });
      
      if (!filePath) return;
      
      // Costruisci il CSV
      const headers = ['Nome', 'Tipo', 'Gruppo', 'Indirizzo', 'Device', 'Area', 'Descrizione'];
      const rows = filteredVariables.map(v => [
        v.name || '',
        getVarTypeName(v.var_type),
        v.var_group || '',
        extractAddress(v.dynamic_settings),
        extractDevice(v.dynamic_settings),
        getAreaTypeName(v.area_type),
        (v.description || '').replace(/"/g, '""')
      ]);
      
      const csvContent = [
        headers.join(';'),
        ...rows.map(row => row.map(cell => `"${cell}"`).join(';'))
      ].join('\n');
      
      // Scrivi il file
      const { writeTextFile } = await import('@tauri-apps/plugin-fs');
      await writeTextFile(filePath, csvContent);
      
      toastMsg = $_('variables.export_success');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
    } catch (e) {
      console.error('Errore esportazione CSV:', e);
      toastMsg = $_('variables.export_error') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }
</script>

<div class="min-h-screen flex flex-col" style="background: linear-gradient(135deg, #c9ffe7 0%, #e9e9ff 70%, #dcecff 100%);">
  
  <!-- TOAST NOTIFICATIONS -->
  {#if showToast}
    <div class="fixed bottom-8 right-8 z-50 px-6 py-3 rounded shadow-lg animate-fadein font-semibold text-white"
      style="background-color: {
        toastType === 'success' ? '#22c55e' : 
        toastType === 'warning' ? '#f59e0b' : 
        toastType === 'info' ? '#3b82f6' : 
        '#ef4444'
      };">
      {toastMsg}
    </div>
  {/if}

  <!-- HEADER CONTENT -->
  <header class="w-full pt-5 px-5 fixed top-0 left-0 right-0 z-10 bg-transparent">
    <div class="w-full bg-white/50 backdrop-blur-sm rounded-lg border border-black/50 p-4 sm:p-2 shadow-lg flex items-center justify-between">
      <button class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded" on:click={goBack} aria-label="{$_('variables.back_to_previous')}">
        ← {$_('variables.back')}
      </button>
      
      <div class="text-center flex-1">
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('variables.title', { values: { name: tableName } })}</h1>
        <p class="text-gray-700 text-sm">
          {$_('variables.showing_variables', { values: { count: filteredVariables.length, total: variables.length } })}
        </p>
      </div>
      
      <div class="flex gap-2 items-center">
        <button 
          class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={exportToCSV}
          disabled={loading || filteredVariables.length === 0}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
          </svg>
          {$_('variables.export_csv')}
        </button>
        <button 
          class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={loadDatabaseData} 
          disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {loading ? $_('variables.reloading') : $_('variables.reload')}
        </button>
      </div>
    </div>
  </header>

  <!-- MAIN CONTENT -->
  <main class="flex-grow pt-5 px-5" style="margin-top: 6rem; margin-bottom: 0.2rem;">
    
    {#if loading}
      <div class="flex justify-center items-center h-64">
        <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-gray-900"></div>
      </div>
    {:else if variables.length === 0}
      <div class="text-center py-20">
        <svg class="mx-auto h-24 w-24 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
        </svg>
        <h3 class="mt-2 text-sm font-medium text-gray-900">{$_('variables.no_variables_found')}</h3>
        <p class="mt-1 text-sm text-gray-500">{$_('variables.no_variables_description')}</p>
      </div>
    {:else}
      <!-- Barra di ricerca e filtri fissa -->
      <div class="fixed top-24 left-5 right-5 z-20 bg-white/95 backdrop-blur-md rounded-lg border border-gray-300/50 p-4">
        <div class="flex items-center gap-4 flex-wrap">
          <!-- Campo di ricerca -->
          <div class="flex-1 min-w-64 relative">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <svg class="h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
              </svg>
            </div>
            <input
              type="text"
              bind:value={searchTerm}
              placeholder="{$_('variables.search_placeholder')}"
              class="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500 sm:text-sm"
            />
          </div>
          
          <!-- Filtro per gruppo -->
          <select
            bind:value={filterGroup}
            class="appearance-none bg-white border border-gray-300 rounded-md px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500">
            <option value="">{$_('variables.all_groups')}</option>
            {#each availableGroups as group}
              <option value={group}>{group}</option>
            {/each}
          </select>
          
          <!-- Filtro per tipo -->
          <select
            bind:value={filterType}
            class="appearance-none bg-white border border-gray-300 rounded-md px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500">
            <option value="">{$_('variables.all_types')}</option>
            {#each availableTypes as type}
              <option value={type}>{getVarTypeName(type)}</option>
            {/each}
          </select>
          
          <!-- Filtro per area -->
          <select
            bind:value={filterAreaType}
            class="appearance-none bg-white border border-gray-300 rounded-md px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-cyan-500 focus:border-cyan-500">
            <option value="">{$_('variables.all_areas')}</option>
            {#each availableAreaTypes as area}
              <option value={area}>{getAreaTypeName(area)}</option>
            {/each}
          </select>
          
          <!-- Statistiche compatte -->
          <div class="flex items-center gap-3">
            <div class="flex items-center bg-cyan-50 rounded-lg px-3 py-2">
              <svg class="h-5 w-5 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
              </svg>
              <div class="ml-2">
                <p class="text-xs font-medium text-cyan-600 uppercase tracking-wide">{$_('variables.variables_label')}</p>
                <p class="text-sm font-semibold text-cyan-900">
                  {filteredVariables.length}/{variables.length}
                </p>
              </div>
            </div>

            <button
              on:click={() => structures.length > 0 && (showStructuresListModal = true)}
              class="flex items-center bg-purple-50 rounded-lg px-3 py-2 {structures.length > 0 ? 'hover:bg-purple-100 cursor-pointer transition-colors' : 'opacity-60 cursor-not-allowed'}">
              <svg class="h-5 w-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
              </svg>
              <div class="ml-2">
                <p class="text-xs font-medium text-purple-600 uppercase tracking-wide">{$_('variables.structures_label')}</p>
                <p class="text-sm font-semibold text-purple-900">{structures.length}</p>
              </div>
            </button>
          </div>
          
          {#if searchTerm || filterGroup || filterType || filterAreaType}
            <button
              on:click={clearFilters}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
              {$_('variables.clear_filters')}
            </button>
          {/if}
        </div>
      </div>

      <!-- Contenitore per la tabella -->
      <div class="w-full" style="margin-top: 70px;">
        
        {#if filteredVariables.length === 0 && (searchTerm || filterGroup || filterType || filterAreaType)}
          <!-- Messaggio nessun risultato -->
          <div class="bg-white/80 backdrop-blur-sm rounded-lg border border-gray-300/50 shadow-lg p-8 text-center">
            <svg class="mx-auto h-16 w-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
            </svg>
            <h3 class="mt-4 text-lg font-medium text-gray-900">{$_('variables.no_results')}</h3>
            <p class="mt-2 text-sm text-gray-500">{$_('variables.no_results_description')}</p>
            <button
              on:click={clearFilters}
              class="mt-4 bg-cyan-500 hover:bg-cyan-600 text-white font-bold py-2 px-4 rounded">
              {$_('variables.show_all')}
            </button>
          </div>
        {:else}
          <div class="bg-white/80 backdrop-blur-sm rounded-lg border border-gray-300/50 shadow-lg overflow-hidden">
            <div class="overflow-x-auto overflow-y-auto" style="scrollbar-width: thin; max-height: calc(100vh - 260px);">
              <table class="min-w-full divide-y divide-gray-200">
                <thead class="sticky top-0 z-20 bg-gray-50/95 backdrop-blur-sm">
                  <tr>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100" on:click={() => toggleSort('name')}>
                      <div class="flex items-center gap-1">
                        {$_('variables.column_name')}
                        {#if sortColumn === 'name'}
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={sortDirection === 'asc' ? 'M5 15l7-7 7 7' : 'M19 9l-7 7-7-7'}></path>
                          </svg>
                        {/if}
                      </div>
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100" on:click={() => toggleSort('var_type')}>
                      <div class="flex items-center gap-1">
                        {$_('variables.column_type')}
                        {#if sortColumn === 'var_type'}
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={sortDirection === 'asc' ? 'M5 15l7-7 7 7' : 'M19 9l-7 7-7-7'}></path>
                          </svg>
                        {/if}
                      </div>
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100" on:click={() => toggleSort('var_group')}>
                      <div class="flex items-center gap-1">
                        {$_('variables.column_group')}
                        {#if sortColumn === 'var_group'}
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={sortDirection === 'asc' ? 'M5 15l7-7 7 7' : 'M19 9l-7 7-7-7'}></path>
                          </svg>
                        {/if}
                      </div>
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {$_('variables.column_address')}
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {$_('variables.column_device')}
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100" on:click={() => toggleSort('area_type')}>
                      <div class="flex items-center gap-1">
                        {$_('variables.column_area')}
                        {#if sortColumn === 'area_type'}
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={sortDirection === 'asc' ? 'M5 15l7-7 7 7' : 'M19 9l-7 7-7-7'}></path>
                          </svg>
                        {/if}
                      </div>
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {$_('variables.column_description')}
                    </th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {$_('variables.column_actions')}
                    </th>
                  </tr>
                </thead>
                <tbody class="bg-white/60 divide-y divide-gray-200">
                  {#each filteredVariables as variable, index}
                    <tr class="hover:bg-cyan-50/50 transition-colors {index % 2 === 0 ? 'bg-white/60' : 'bg-gray-50/40'}">
                      <td class="px-4 py-3 text-sm font-medium text-gray-900">
                        <div class="max-w-xs truncate" title={variable.name}>
                          {#if searchTerm && variable.name && variable.name.toLowerCase().includes(searchTerm.toLowerCase())}
                            {@html variable.name.replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-0.5 rounded">$1</mark>')}
                          {:else}
                            {variable.name}
                          {/if}
                        </div>
                      </td>
                      <td class="px-4 py-3 text-sm">
                        <span class="px-2 py-1 text-xs font-medium rounded-full {getTypeColor(variable.var_type)}">
                          {getVarTypeName(variable.var_type)}
                        </span>
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        {variable.var_group || '-'}
                      </td>
                      <td class="px-4 py-3 text-sm font-mono text-gray-700">
                        {extractAddress(variable.dynamic_settings)}
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        {extractDevice(variable.dynamic_settings)}
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        {getAreaTypeName(variable.area_type)}
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        <div class="max-w-xs truncate" title={variable.description || ''}>
                          {#if searchTerm && variable.description && variable.description.toLowerCase().includes(searchTerm.toLowerCase())}
                            {@html variable.description.replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-0.5 rounded">$1</mark>')}
                          {:else}
                            {variable.description || '-'}
                          {/if}
                        </div>
                      </td>
                      <td class="px-4 py-3 text-sm">
                        <button
                          on:click={() => showVariableDetails(variable)}
                          class="bg-cyan-500 hover:bg-cyan-600 text-white text-xs font-bold py-1 px-2 rounded flex items-center gap-1">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                          </svg>
                          {$_('variables.view_details')}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<!-- Modal lista strutture -->
{#if showStructuresListModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" on:click={() => showStructuresListModal = false} on:keypress={(e) => e.key === 'Escape' && (showStructuresListModal = false)} role="dialog" tabindex="-1">
    <div class="bg-white rounded-lg max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden" on:click|stopPropagation role="document">
      <div class="p-4 border-b border-gray-200 flex justify-between items-center bg-purple-50">
        <h3 class="text-lg font-semibold text-purple-900 flex items-center gap-2">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
          </svg>
          {$_('variables.structures_available')} ({structures.length})
        </h3>
        <button on:click={() => showStructuresListModal = false} class="text-gray-500 hover:text-gray-700">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      
      <div class="p-4 overflow-y-auto max-h-[60vh]">
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
          {#each structures as structure}
            <button
              on:click={() => { showStructuresListModal = false; showStructureDetails(structure); }}
              class="bg-purple-100 hover:bg-purple-200 text-purple-800 text-sm font-medium px-3 py-2 rounded-lg transition-colors text-left flex items-center gap-2">
              <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
              </svg>
              <span class="truncate">{structure.name}</span>
            </button>
          {/each}
        </div>
      </div>
      
      <div class="p-4 border-t border-gray-200 flex justify-end bg-gray-50">
        <button on:click={() => showStructuresListModal = false} class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded">
          {$_('variables.close')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Modal dettagli struttura -->
{#if showStructureModal && selectedStructure}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" on:click={() => showStructureModal = false} on:keypress={(e) => e.key === 'Escape' && (showStructureModal = false)} role="dialog" tabindex="-1">
    <div class="bg-white rounded-lg  max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden" on:click|stopPropagation role="document">
      <div class="p-4 border-b border-gray-200 flex justify-between items-center bg-purple-50">
        <h3 class="text-lg font-semibold text-purple-900 flex items-center gap-2">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
          </svg>
          {$_('variables.structure_details')}: {selectedStructure.name}
        </h3>
        <button on:click={() => showStructureModal = false} class="text-gray-500 hover:text-gray-700">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      
      <div class="p-4 overflow-y-auto max-h-[60vh]">
        {#if selectedStructure.description}
          <p class="text-sm text-gray-600 mb-4 bg-gray-50 p-2 rounded">{selectedStructure.description}</p>
        {/if}
        
        <h4 class="font-semibold text-gray-800 mb-2 flex items-center gap-2">
          <svg class="w-4 h-4 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"></path>
          </svg>
          {$_('variables.structure_members')} ({structureMembers.length})
        </h4>
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-purple-50">
            <tr>
              <th class="px-4 py-2 text-left text-xs font-medium text-purple-700 uppercase">{$_('variables.member_name')}</th>
              <th class="px-4 py-2 text-left text-xs font-medium text-purple-700 uppercase">{$_('variables.member_type')}</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200">
            {#each structureMembers as member, i}
              <tr class="{i % 2 === 0 ? 'bg-white' : 'bg-purple-50/30'}">
                <td class="px-4 py-2 text-sm text-gray-900 font-medium">{member.member_name}</td>
                <td class="px-4 py-2 text-sm">
                  <span class="px-2 py-1 text-xs font-medium rounded-full {getTypeColor(member.member_type)}">
                    {getVarTypeName(member.member_type)}
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      
      <div class="p-4 border-t border-gray-200 flex justify-end bg-gray-50">
        <button on:click={() => showStructureModal = false} class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded">
          {$_('variables.close')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Modal dettagli variabile -->
{#if showVariableModal && selectedVariable}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" on:click={() => showVariableModal = false} on:keypress={(e) => e.key === 'Escape' && (showVariableModal = false)} role="dialog" tabindex="-1">
    <div class="bg-white rounded-lg  max-w-lg w-full mx-4 max-h-[80vh] overflow-hidden" on:click|stopPropagation role="document">
      <div class="p-4 border-b border-gray-200 flex justify-between items-center bg-cyan-50">
        <h3 class="text-lg font-semibold text-cyan-900 flex items-center gap-2">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
          </svg>
          {$_('variables.variable_details')}
        </h3>
        <button on:click={() => showVariableModal = false} class="text-gray-500 hover:text-gray-700">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      
      <div class="p-4 overflow-y-auto max-h-[60vh]">
        <div class="space-y-4">
          <!-- Nome -->
          <div class="bg-gray-50 p-3 rounded-lg">
            <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_name')}</p>
            <p class="text-lg font-semibold text-gray-900">{selectedVariable.name}</p>
          </div>
          
          <!-- Tipo e Area -->
          <div class="grid grid-cols-2 gap-4">
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_type')}</p>
              <span class="px-2 py-1 text-sm font-medium rounded-full {getTypeColor(selectedVariable.var_type)}">
                {getVarTypeName(selectedVariable.var_type)}
              </span>
            </div>
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_area')}</p>
              <p class="text-sm text-gray-900">{getAreaTypeName(selectedVariable.area_type)}</p>
            </div>
          </div>
          
          <!-- Gruppo -->
          <div class="bg-gray-50 p-3 rounded-lg">
            <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_group')}</p>
            <p class="text-sm text-gray-900">{selectedVariable.var_group || '-'}</p>
          </div>
          
          <!-- Indirizzo e Device -->
          <div class="grid grid-cols-2 gap-4">
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_address')}</p>
              <p class="text-sm font-mono text-gray-900">{extractAddress(selectedVariable.dynamic_settings)}</p>
            </div>
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_device')}</p>
              <p class="text-sm text-gray-900">{extractDevice(selectedVariable.dynamic_settings)}</p>
            </div>
          </div>
          
          <!-- Descrizione -->
          {#if selectedVariable.description}
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.column_description')}</p>
              <p class="text-sm text-gray-900">{selectedVariable.description}</p>
            </div>
          {/if}
          
          <!-- Dynamic Settings completo -->
          {#if selectedVariable.dynamic_settings}
            <div class="bg-gray-50 p-3 rounded-lg">
              <p class="text-xs font-medium text-gray-500 uppercase">{$_('variables.dynamic_settings')}</p>
              <p class="text-xs font-mono text-gray-700 break-all">{selectedVariable.dynamic_settings}</p>
            </div>
          {/if}
        </div>
      </div>
      
      <div class="p-4 border-t border-gray-200 flex justify-end bg-gray-50">
        <button on:click={() => showVariableModal = false} class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded">
          {$_('variables.close')}
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
