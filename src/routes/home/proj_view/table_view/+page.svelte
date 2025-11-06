<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';

  let tableName = '';
  let records = [];
  let columns = [];
  let loading = false;
  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';
  let searchTerm = '';
  let filteredRecords = [];
  let showConfirmModal = false;
  let confirmModalTitle = '';
  let confirmModalMessage = '';
  let confirmModalType = 'info'; // 'info', 'confirm', 'error'
  let unusedKeysCount = 0;
  let isTranslating = false;
  // Inline edit state for manual editing of language cell values
  let editingCell = { id: null, column: null };
  let editValue = '';
  // Delete confirmation state
  let showDeleteModal = false;
  let recordToDelete = null;

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
      // Carica i record e le colonne
      records = await invoke('get_records', { tableName: tableName });
      columns = await invoke('get_table_columns', { tableName: tableName });
      
      console.log('Loaded records:', records);
      console.log('Loaded columns:', columns);
      console.log('Visible columns:', getVisibleColumns(columns));
      console.log('Sample record:', records.length > 0 ? records[0] : 'No records');
      console.log('Language columns count:', getVisibleColumns(columns).filter(col => {
        const techColumns = ['id', 'key', 'keys_project', 'image_path', 'order', 'project_id', 'file_path', 'source_file'];
        return !techColumns.includes(col.toLowerCase()) && !col.toLowerCase().includes('_id') && !col.toLowerCase().includes('path');
      }).length);
    } catch (e) {
      console.error('Errore nel caricamento dei dati:', e);
      toastMsg = $_('database.error_loading_data') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
    loading = false;
  }

  // Filtra le colonne da mostrare (nascondi quelle interne/tecniche) e riordina
  function getVisibleColumns(columns) {
    const hiddenColumns = ['created_at', 'updated_at', 'timestamp', 'created', 'updated'];
    const filteredColumns = columns.filter(col => !hiddenColumns.includes(col.toLowerCase()));
    
    // Riordina le colonne con "keys_project" come terza colonna
    const orderedColumns = [];
    
    // Prima le colonne prioritarie nell'ordine desiderato
    if (filteredColumns.includes('id')) orderedColumns.push('id');
    if (filteredColumns.includes('key')) orderedColumns.push('key');
    if (filteredColumns.includes('keys_project')) orderedColumns.push('keys_project');
    
    // Poi tutte le altre colonne (escludendo quelle già aggiunte)
    filteredColumns.forEach(col => {
      if (!['id', 'key', 'keys_project'].includes(col)) {
        orderedColumns.push(col);
      }
    });
    
    return orderedColumns;
  }

  // Filtra i record in base al termine di ricerca
  function filterRecords(records, searchTerm) {
    if (!searchTerm.trim()) {
      return records;
    }
    
    const term = searchTerm.toLowerCase();
    return records.filter(record => {
      return getVisibleColumns(columns).some(column => {
        const value = record[column];
        return value && value.toString().toLowerCase().includes(term);
      });
    });
  }

  // Reattiva per aggiornare i record filtrati
  $: filteredRecords = filterRecords(records, searchTerm);
  $: visibleColumns = getVisibleColumns(columns);
  
  function clearSearch() {
    searchTerm = '';
  }

  function goBack() {
    window.history.back();
  }

  async function removeUnusedKeys() {
    try {
      // Controlla se ci sono chiavi di progetto (significa che è stata fatta la ricerca)
      const projectKeysWithStatus = await invoke('get_project_keys_with_status', { projectName: tableName });
      
      if (!projectKeysWithStatus || projectKeysWithStatus.length === 0) {
        // Non sono state caricate chiavi di progetto
        confirmModalTitle = $_('database.search_keys_required');
        confirmModalMessage = $_('database.search_keys_required_message');
        confirmModalType = 'info';
        showConfirmModal = true;
        return;
      }

      // Conta le chiavi inutilizzate (quelle che non hanno keys_project)
      const unusedKeys = records.filter(record => !record.keys_project && record.key);
      unusedKeysCount = unusedKeys.length;

      if (unusedKeysCount === 0) {
        confirmModalTitle = $_('database.no_keys_to_remove');
        confirmModalMessage = $_('database.no_unused_keys');
        confirmModalType = 'info';
        showConfirmModal = true;
        return;
      }

      // Mostra conferma di eliminazione
      confirmModalTitle = $_('database.confirm_deletion');
      confirmModalMessage = $_('database.confirm_delete_unused_keys', { 
        values: { 
          count: unusedKeysCount,
          plural: unusedKeysCount > 1 ? 'i' : '',
          plural2: unusedKeysCount > 1 ? 'e' : ''
        } 
      });
      confirmModalType = 'confirm';
      showConfirmModal = true;

    } catch (error) {
      console.error('Errore nel controllo delle chiavi:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_checking_keys') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
    }
  }

  async function confirmRemoveUnusedKeys() {
    try {
      // Chiama il backend per eliminare le chiavi inutilizzate
      const result = await invoke('remove_unused_keys', { projectName: tableName });
      
      confirmModalTitle = $_('database.deletion_completed');
      confirmModalMessage = result;
      confirmModalType = 'info';
      
      // Ricarica i dati per aggiornare la visualizzazione
      await loadDatabaseData();
      
    } catch (error) {
      console.error('Errore nell\'eliminazione delle chiavi:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_deleting_keys') + ' ' + error;
      confirmModalType = 'error';
    }
  }

  function closeConfirmModal() {
    showConfirmModal = false;
  }

  async function addKeyToTranslations(recordId, keyValue) {
    try {
      // Aggiorna il record impostando key = keys_project
      await invoke('update_record', { 
        tableName: tableName, 
        id: recordId, 
        updates: { key: keyValue } 
      });

      // Aggiorna solo il record specifico nell'array locale invece di ricaricare tutto
      const recordIndex = records.findIndex(record => record.id === recordId);
      if (recordIndex !== -1) {
        records[recordIndex].key = keyValue;
        records = [...records]; // Trigger reattività Svelte
      }

      // Mostra toast di successo
      showToast = true;
      toastMsg = $_('database.key_added_translations');
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);

    } catch (error) {
      console.error('Errore nell\'aggiunta della chiave:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_adding_key_translations') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
    }
  }

  // Funzione per copiare la chiave nella colonna lingua
  async function addFromKey(recordId, languageColumn, keyValue) {
    try {
      const updates = {};
      updates[languageColumn] = keyValue;
      
      await invoke('update_record', { 
        tableName: tableName, 
        id: recordId, 
        updates: updates
      });

      // Aggiorna solo il record specifico nell'array locale invece di ricaricare tutto
      const recordIndex = records.findIndex(record => record.id === recordId);
      if (recordIndex !== -1) {
        records[recordIndex][languageColumn] = keyValue;
        records = [...records]; // Trigger reattività Svelte
      }

      // Mostra toast di successo
      showToast = true;
      toastMsg = $_('database.value_added_from_key', { values: { column: languageColumn } });
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);

    } catch (error) {
      console.error('Errore nell\'aggiunta dalla chiave:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_adding_from_key') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
    }
  }

  // Funzione helper per identificare le colonne lingua
  function isLanguageColumn(column) {
    return visibleColumns.filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'image_path' && col !== 'order').includes(column);
  }

  // Funzione per eliminare il valore di una colonna lingua
  async function clearLanguageValue(recordId, languageColumn) {
    try {
      const updates = {};
      updates[languageColumn] = ""; // Usa stringa vuota invece di null
      
      await invoke('update_record', { 
        tableName: tableName, 
        id: recordId, 
        updates: updates
      });

      // Aggiorna solo il record specifico nell'array locale invece di ricaricare tutto
      const recordIndex = records.findIndex(record => record.id === recordId);
      if (recordIndex !== -1) {
        records[recordIndex][languageColumn] = ""; // Usa stringa vuota anche qui
        records = [...records]; // Trigger reattività Svelte
      }

      // Mostra toast di successo
      showToast = true;
      toastMsg = $_('database.value_deleted_from', { values: { column: languageColumn } });
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);

    } catch (error) {
      console.error('Errore nell\'eliminazione del valore:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_deleting_value') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
    }
  }

  // Funzione per tradurre usando un servizio online
  async function translateText(recordId, languageColumn, sourceText, sourceLang, targetLang) {
    if (isTranslating) return;
    
    isTranslating = true;
    try {
      // Chiama la funzione backend per la traduzione
      const translatedText = await invoke('translate_text', {
        text: sourceText,
        sourceLang: sourceLang,
        targetLang: targetLang
      });
      
      const updates = {};
      updates[languageColumn] = translatedText;
      
      await invoke('update_record', { 
        tableName: tableName, 
        id: recordId, 
        updates: updates
      });

      // Aggiorna solo il record specifico nell'array locale invece di ricaricare tutto
      const recordIndex = records.findIndex(record => record.id === recordId);
      if (recordIndex !== -1) {
        records[recordIndex][languageColumn] = translatedText;
        records = [...records]; // Trigger reattività Svelte
      }
      
      // Mostra messaggio di successo
      showToast = true;
      toastMsg = $_('database.text_translated', { values: { column: languageColumn } });
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);
      
    } catch (error) {
      console.error('Errore nella traduzione:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_translation') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
    } finally {
      isTranslating = false;
    }
  }



  // Funzione per determinare quale pulsante mostrare per una colonna lingua
  function getTranslationAction(record, languageColumn) {
    const keyValue = record['key'];
    const currentValue = record[languageColumn];
    
    // Se la colonna lingua già ha un valore, non mostrare pulsanti
    if (currentValue && currentValue.trim()) {
      return null;
    }
    
    // Se non c'è una chiave, non possiamo fare nulla
    if (!keyValue || !keyValue.trim()) {
      return null;
    }
    
    // Trova le lingue che hanno già traduzioni per questo record
    const availableTranslations = visibleColumns
      .filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'image_path' && col !== 'order')
      .filter(col => record[col] && record[col].trim())
      .filter(col => col !== languageColumn);

    if (availableTranslations.length === 0) {
      // Nessuna traduzione disponibile, offri "Aggiungi da chiave"
      return {
        type: 'from_key',
        text: $_('database.add_from_key'),
        action: () => addFromKey(record.id, languageColumn, keyValue)
      };
    } else {
      // Ci sono traduzioni disponibili, offri traduzione dalla prima lingua disponibile
      const sourceLang = availableTranslations[0];
      return {
        type: 'translate',
        text: $_('database.translate_from', { values: { language: sourceLang } }),
        action: () => translateText(record.id, languageColumn, record[sourceLang], sourceLang, languageColumn)
      };
    }
  }

  // Funzione per verificare se una colonna ha valori vuoti
  function hasEmptyValues(column) {
    if (!isLanguageColumn(column)) return false;
    return filteredRecords.some(record => 
      (!record[column] || !record[column].trim()) && 
      record['key'] && record['key'].trim()
    );
  }

  // Funzione per contare quanti valori vuoti ha una colonna
  function countEmptyValues(column) {
    if (!isLanguageColumn(column)) return 0;
    return filteredRecords.filter(record => 
      (!record[column] || !record[column].trim()) && 
      record['key'] && record['key'].trim()
    ).length;
  }

  // Funzione per tradurre tutti i valori vuoti di una colonna
  async function translateAllEmpty(column) {
    if (isTranslating) return;
    
    const emptyRecords = filteredRecords.filter(record => 
      (!record[column] || !record[column].trim()) && 
      record['key'] && record['key'].trim()
    );
    
    if (emptyRecords.length === 0) {
      showToast = true;
      toastMsg = $_('database.no_empty_values', { values: { column: column } });
      toastType = 'info';
      setTimeout(() => showToast = false, 3000);
      return;
    }

    isTranslating = true;
    
    try {
      showToast = true;
      toastMsg = $_('database.translating_values', { values: { count: emptyRecords.length, column: column } });
      toastType = 'info';
      setTimeout(() => showToast = false, 3000);

      let successCount = 0;
      let errorCount = 0;

      for (const record of emptyRecords) {
        try {
          // Trova la migliore fonte per la traduzione
          const availableTranslations = visibleColumns
            .filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'image_path' && col !== 'order')
            .filter(col => record[col] && record[col].trim())
            .filter(col => col !== column);

          let sourceText = '';
          let sourceLang = '';

          if (availableTranslations.length > 0) {
            // Usa la prima traduzione disponibile
            sourceLang = availableTranslations[0];
            sourceText = record[sourceLang];
          } else {
            // Usa la chiave come fallback
            sourceText = record['key'];
            sourceLang = 'auto';
          }

          // Traduci il testo
          const translatedText = await invoke('translate_text', {
            text: sourceText,
            sourceLang: sourceLang,
            targetLang: column
          });
          
          const updates = {};
          updates[column] = translatedText;
          
          await invoke('update_record', { 
            tableName: tableName, 
            id: record.id, 
            updates: updates
          });

          // Aggiorna il record nell'array locale
          const recordIndex = records.findIndex(r => r.id === record.id);
          if (recordIndex !== -1) {
            records[recordIndex][column] = translatedText;
          }
          
          successCount++;
          
        } catch (error) {
          console.error(`Errore nella traduzione del record ${record.id}:`, error);
          errorCount++;
        }
      }

      // Trigger reattività Svelte
      records = [...records];

      // Mostra risultato finale
      showToast = true;
      if (errorCount === 0) {
        toastMsg = $_('database.translation_completed', { values: { count: successCount, column: column } });
        toastType = 'success';
      } else {
        toastMsg = $_('database.partial_translation', { values: { success: successCount, errors: errorCount, column: column } });
        toastType = 'warning';
      }
      setTimeout(() => showToast = false, 5000);
      
    } catch (error) {
      console.error('Errore nella traduzione multipla:', error);
      showToast = true;
      toastMsg = $_('database.error_multiple_translation') + ' ' + error;
      toastType = 'error';
      setTimeout(() => showToast = false, 5000);
    } finally {
      isTranslating = false;
    }
  }

  // Inline edit helpers
  function startEdit(recordId, column, initial = '') {
    editingCell = { id: recordId, column };
    editValue = initial || '';
  }

  async function saveEdit(recordId, column) {
    try {
      const updates = {};
      updates[column] = editValue;

      await invoke('update_record', {
        tableName: tableName,
        id: recordId,
        updates: updates
      });

      // Aggiorna solo il record specifico nell'array locale invece di ricaricare tutto
      const recordIndex = records.findIndex(r => r.id === recordId);
      if (recordIndex !== -1) {
        records[recordIndex][column] = editValue;
        records = [...records];
      }

      showToast = true;
      toastMsg = $_('database.value_saved', { values: { column } });
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);

      // reset edit state
      editingCell = { id: null, column: null };
      editValue = '';
    } catch (error) {
      console.error('Errore nel salvataggio manuale:', error);
      showToast = true;
      toastMsg = $_('database.save_error') + error;
      toastType = 'error';
      setTimeout(() => showToast = false, 5000);
    }
  }

  function cancelEdit() {
    editingCell = { id: null, column: null };
    editValue = '';
  }

  // Funzione per mostrare il popup di conferma eliminazione (o eliminare direttamente se non ha traduzioni)
  function confirmDeleteRecord(recordId) {
    const record = records.find(r => r.id === recordId);
    if (!record) return;
    
    // Controlla se il record ha traduzioni nelle colonne lingua
    const languageColumns = visibleColumns.filter(col => isLanguageColumn(col));
    const hasTranslations = languageColumns.some(col => record[col] && record[col].trim());
    
    // Controlla se il record ha una key
    const hasKey = record['key'] && record['key'].trim();
    
    if (!hasTranslations && !hasKey) {
      // Nessuna traduzione e nessuna key presente, elimina direttamente senza conferma
      recordToDelete = recordId;
      deleteRecord();
    } else {
      // Ha traduzioni o ha una key, mostra conferma
      recordToDelete = recordId;
      showDeleteModal = true;
    }
  }

  // Funzione per eliminare completamente un record
  async function deleteRecord() {
    if (!recordToDelete) return;
    
    try {
      await invoke('delete_record', { 
        tableName: tableName, 
        id: recordToDelete 
      });

      // Rimuovi il record dall'array locale
      records = records.filter(record => record.id !== recordToDelete);

      // Mostra toast di successo
      showToast = true;
      toastMsg = $_('database.record_deleted');
      toastType = 'success';
      setTimeout(() => showToast = false, 3000);

      // Reset state
      showDeleteModal = false;
      recordToDelete = null;

    } catch (error) {
      console.error('Errore nell\'eliminazione del record:', error);
      confirmModalTitle = $_('database.error');
      confirmModalMessage = $_('database.error_deleting_record') + ' ' + error;
      confirmModalType = 'error';
      showConfirmModal = true;
      showDeleteModal = false;
      recordToDelete = null;
    }
  }

  function cancelDeleteRecord() {
    showDeleteModal = false;
    recordToDelete = null;
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
      <button class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded" on:click={goBack} aria-label="{$_('database.back_to_previous')}">
        ← {$_('database.back')}
      </button>
      
      <div class="text-center flex-1">
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('database.database_title', { values: { name: tableName } })}</h1>
        <p class="text-gray-700 text-sm">
          {$_('database.records_count', { values: { filtered: filteredRecords.length, total: records.length } })}
          {searchTerm ? $_('database.filtered_by', { values: { term: searchTerm } }) : ''}
        </p>
      </div>
      
      <div class="flex gap-2 items-center">
        <button 
          class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={removeUnusedKeys}
          disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
          </svg>
          {$_('database.remove_unused_keys')}
        </button>
        <button 
          class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={loadDatabaseData} 
          disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {loading ? $_('database.reloading') : $_('database.reload')}
        </button>
      </div>
    </div>
  </header>

  <!-- MAIN CONTENT -->
  <main class="flex-grow pt-5 px-5 mb-16" style="margin-top: 6rem; margin-bottom: 5rem;">
    
    {#if loading}
      <div class="flex justify-center items-center h-64">
        <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-gray-900"></div>
      </div>
    {:else if records.length === 0}
      <div class="text-center py-20">
        <svg class="mx-auto h-24 w-24 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.5a2.5 2.5 0 00-2.5 2.5v0a2.5 2.5 0 01-2.5 2.5H9a2.5 2.5 0 01-2.5-2.5v0A2.5 2.5 0 014 13h2.5"></path>
        </svg>
        <h3 class="mt-2 text-sm font-medium text-gray-900">{$_('database.no_records_found')}</h3>
        <p class="mt-1 text-sm text-gray-500">{$_('database.no_translation_records')}</p>
      </div>
    {:else}
      <!-- Barra di ricerca e statistiche fissa -->
      <div class="fixed top-24 left-5 right-5 z-20 bg-white/95 backdrop-blur-md rounded-lg border border-gray-300/50 shadow-xl p-4">
          <!-- Barra di ricerca con statistiche integrate -->
          <div class="flex items-center gap-4 mb-1">
            <!-- Campo di ricerca -->
            <div class="flex-1 relative">
              <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <svg class="h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                </svg>
              </div>
              <input
                type="text"
                bind:value={searchTerm}
                placeholder="{$_('database.search_placeholder')}"
                class="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              />
            </div>
            
            <!-- Statistiche compatte -->
            <div class="flex items-center gap-3">
              <div class="flex items-center bg-blue-50 rounded-lg px-3 py-2">
                <div class="flex-shrink-0">
                  <svg class="h-5 w-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                  </svg>
                </div>
                <div class="ml-2">
                  <p class="text-xs font-medium text-blue-600 uppercase tracking-wide">
                    {searchTerm ? $_('database.filtered') : $_('database.records')}
                  </p>
                  <p class="text-sm font-semibold text-blue-900">
                    {searchTerm ? `${filteredRecords.length}/${records.length}` : records.length}
                  </p>
                </div>
              </div>

              <!-- Statistiche chiavi utilizzate -->
              <div class="flex items-center bg-green-50 rounded-lg px-3 py-2">
                <div class="flex-shrink-0">
                  <svg class="h-5 w-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0 3.866-3.582 7-8 7m8-7c0-3.866 3.582-7 8-7m-8 7v10m0 0H4m8 0h8"></path>
                  </svg>
                </div>
                <div class="ml-2">
                  <p class="text-xs font-medium text-green-600 uppercase tracking-wide">{$_('database.used_keys')}</p>
                  <p class="text-sm font-semibold text-green-900">{filteredRecords.filter(r => r['keys_project'] && r['keys_project'] === r['key']).length}</p>
                </div>
              </div>

              <div class="flex items-center bg-purple-50 rounded-lg px-3 py-2">
                <div class="flex-shrink-0">
                  <svg class="h-5 w-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 6l3 1m0 0l-3 9a5.002 5.002 0 006.001 0M6 7l3 9M6 7l6-2m6 2l3-1m-3 1l-3 9a5.002 5.002 0 006.001 0M18 7l3 9m-3-9l-6-2m0-2v2m0 16V5m0 16H9m3 0h3"></path>
                  </svg>
                </div>
                <div class="ml-2">
                  <p class="text-xs font-medium text-purple-600 uppercase tracking-wide">{$_('database.languages')}</p>
                  <p class="text-sm font-semibold text-purple-900">{visibleColumns.filter(col => {
                    const techColumns = ['id', 'key', 'keys_project', 'image_path', 'order', 'project_id', 'file_path', 'source_file'];
                    return !techColumns.includes(col.toLowerCase()) && !col.toLowerCase().includes('_id') && !col.toLowerCase().includes('path');
                  }).length}</p>
                </div>
              </div>
            </div>
            
            {#if searchTerm}
              <button
                on:click={clearSearch}
                class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2"
                aria-label="{$_('database.clear_search')}">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
                {$_('database.clear')}
              </button>
            {/if}
          </div>
          
          {#if searchTerm}
            <div class="mt-1 text-sm text-gray-600 bg-yellow-50 rounded-lg p-1">
              <svg class="inline w-4 h-4 mr-1 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              {$_('database.search_results', { values: { count: filteredRecords.length, term: searchTerm } })}
            </div>
          {/if}
        </div>

        <!-- Contenitore per la tabella -->
        <div class="w-full pb-32" style="margin-top: 80px;">
          
          <!-- Tabella dei record -->
          {#if filteredRecords.length === 0 && searchTerm}
            <!-- Messaggio nessun risultato -->
            <div class="bg-white/80 backdrop-blur-sm rounded-lg border border-gray-300/50 shadow-lg p-8 text-center">
              <svg class="mx-auto h-16 w-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
              </svg>
              <h3 class="mt-4 text-lg font-medium text-gray-900">{$_('database.no_results')}</h3>
              <p class="mt-2 text-sm text-gray-500">
                {$_('database.no_results_message', { values: { term: searchTerm } })}
              </p>
              <button
                on:click={clearSearch}
                class="mt-4 bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
                {$_('database.show_all_records')}
              </button>
            </div>
          {:else}
            <div class="bg-white/80 backdrop-blur-sm rounded-lg border border-gray-300/50 shadow-lg overflow-hidden">
              <div class="overflow-x-auto overflow-y-auto" style="scrollbar-width: thin; max-height: calc(100vh - 280px);">
                <table class="min-w-full divide-y divide-gray-200">
                  <thead class="sticky top-0 z-20 bg-gray-50/80 backdrop-blur-sm">
                    <tr>
                      {#each visibleColumns as column, columnIndex}
                        <th class="py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider {columnIndex % 2 === 0 ? 'bg-gray-50/80' : 'bg-blue-100/60'} {column === 'keys_project' ? 'px-2 w-40' : 'px-6'}">
                          <div class="flex items-center justify-between">
                            <span>
                              {column === 'id' ? 'ID' : 
                               column === 'key' ? $_('database.column_key') :
                               column === 'keys_project' ? $_('database.column_project') :
                               column === 'image_path' ? $_('database.column_image') :
                               column === 'order' ? $_('database.column_order') :
                               column}
                            </span>
                            
                            {#if isLanguageColumn(column) && hasEmptyValues(column)}
                              <button
                                on:click={() => translateAllEmpty(column)}
                                disabled={isTranslating}
                                class="ml-2 bg-green-500 hover:bg-green-600 disabled:bg-gray-400 text-white text-xs font-bold py-1 px-2 rounded flex items-center gap-1 transition-colors"
                                title="{$_('database.translate_all_empty', { values: { count: countEmptyValues(column), column } })}">
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                                </svg>
                                {isTranslating ? $_('database.translating') : $_('database.translate_all_count', { values: { count: countEmptyValues(column) } })}
                              </button>
                            {/if}
                          </div>
                        </th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody class="bg-white/60 divide-y divide-gray-200">
                {#each filteredRecords as record, index}
                  <tr class="group hover:bg-gray-50/80 transition-colors">
                    {#each visibleColumns as column, columnIndex}
                      <td class="py-4 text-sm text-gray-900 {columnIndex % 2 === 0 ? 'bg-white/60' : 'bg-blue-50/40'} {column === 'keys_project' ? 'px-2 w-40' : 'px-6 whitespace-nowrap'}">
                        {#if column === 'keys_project'}
                          <!-- Visualizzazione speciale per la colonna keys_project -->
                          <div class="flex items-center">
                            {#if record[column] && record[column] === record['key']}
                              <div class="flex items-center text-green-600 font-medium">
                                <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                </svg>
                                {$_('database.present')}
                              </div>
                            {:else if record[column] && !record['key']}
                              <!-- Chiave presente nel progetto ma non tradotta -->
                              <div class="max-w-48 truncate text-orange-600 font-medium" title={record[column] || ''}>
                                {#if searchTerm && record[column].toString().toLowerCase().includes(searchTerm.toLowerCase())}
                                  {@html record[column].toString().replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-1 rounded">$1</mark>')}
                                {:else}
                                  {record[column]}
                                {/if}
                              </div>
                            {:else}
                              <div class="flex items-center text-red-600 font-medium">
                                <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                </svg>
                                {$_('database.not_present')}
                              </div>
                            {/if}
                          </div>
                        {:else if column === 'key'}
                          <!-- Visualizzazione speciale per la colonna key -->
                          <div class="max-w-xs" title={record[column] || ''}>
                            {#if !record[column] && record['keys_project']}
                              <!-- Key non presente ma keys_project sì -->
                              <div class="flex items-center justify-between">
                                <div class="flex items-center text-red-600 font-medium">
                                  <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                  </svg>
                                  {$_('database.not_present_in_translation')}
                                </div>
                                <div class="flex items-center justify-end gap-1">
                                  <button
                                    on:click={() => addKeyToTranslations(record.id, record['keys_project'])}
                                    class="bg-green-500 hover:bg-green-600 text-white text-xs font-bold py-1 px-2 rounded flex items-center gap-1"
                                    title="{$_('database.add_key_to_translations')}">
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                                    </svg>
                                    {$_('database.add')}
                                  </button>
                                  <button
                                    on:click={() => confirmDeleteRecord(record.id)}
                                    class="bg-red-500 hover:bg-red-600 text-white text-xs font-bold py-1 px-2 rounded"
                                    title="{$_('database.delete_record')}">
                                    {$_('database.key_column.delete')}
                                  </button>
                                </div>
                              </div>
                            {:else if searchTerm && record[column] && record[column].toString().toLowerCase().includes(searchTerm.toLowerCase())}
                              <div class="truncate">
                                {@html record[column].toString().replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-1 rounded">$1</mark>')}
                              </div>
                            {:else if !record[column] && !record['keys_project']}
                              <!-- Record completamente vuoto -->
                              <div class="flex items-center justify-between">
                                <span class="text-gray-400">{$_('database.empty_record')}</span>
                                <div class="flex justify-end">
                                  <button
                                    on:click={() => confirmDeleteRecord(record.id)}
                                    class="bg-red-500 hover:bg-red-600 text-white text-xs font-bold py-1 px-2 rounded"
                                    title="{$_('database.delete_empty_record')}">
                                    {$_('database.delete_empty_record')}
                                  </button>
                                </div>
                              </div>
                            {:else}
                              <div class="flex items-center justify-between">
                                <div class="truncate">
                                  {record[column] || '—'}
                                </div>
                                {#if record[column]}
                                  <div class="flex justify-end">
                                    <button
                                      on:click={() => confirmDeleteRecord(record.id)}
                                      class="bg-red-500 hover:bg-red-600 text-white text-xs font-bold py-1 px-2 rounded"
                                      title="{$_('database.delete_record')}">
                                      {$_('database.key_column.delete')}
                                    </button>
                                  </div>
                                {/if}
                              </div>
                            {/if}
                          </div>
                        {:else}
                          <!-- Visualizzazione per colonne lingua e altre colonne -->
                          {#if isLanguageColumn(column)}
                            <!-- Questa è una colonna lingua -->
                            <div class="max-w-xs" title={record[column] || ''}>
                              {#if record[column] && record[column].trim()}
                                <!-- La traduzione esiste già -->
                                {#if editingCell.id === record.id && editingCell.column === column}
                                  <div class="flex items-center gap-2">
                                    <input
                                      class="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                                      bind:value={editValue}
                                      placeholder="{$_('database.enter_value')}" />
                                    <button
                                      on:click={() => saveEdit(record.id, column)}
                                      class="bg-green-500 hover:bg-green-600 text-white text-xs font-bold py-1 px-2 rounded">
                                      {$_('database.save')}
                                    </button>
                                    <button
                                      on:click={cancelEdit}
                                      class="bg-gray-300 hover:bg-gray-400 text-gray-800 text-xs font-bold py-1 px-2 rounded">
                                      {$_('database.cancel')}
                                    </button>
                                  </div>
                                {:else}
                                  <div class="flex items-center gap-2">
                                    <div class="truncate flex-1">
                                      {#if searchTerm && record[column].toString().toLowerCase().includes(searchTerm.toLowerCase())}
                                        {@html record[column].toString().replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-1 rounded">$1</mark>')}
                                      {:else}
                                        {record[column]}
                                      {/if}
                                    </div>
                                    <div class="flex items-center justify-end gap-1 flex-shrink-0">
                                      <button
                                        on:click={() => startEdit(record.id, column, record[column] || '')}
                                        class="bg-yellow-400 hover:bg-yellow-500 text-white text-xs font-bold py-1 px-2 rounded"
                                        title="{$_('database.edit_value')}">
                                        {$_('database.edit')}
                                      </button>
                                      <button
                                        on:click={() => clearLanguageValue(record.id, column)}
                                        class="bg-red-500 hover:bg-red-600 text-white text-xs font-bold py-1 px-2 rounded flex-shrink-0 flex items-center gap-1"
                                        title="{$_('database.delete_value')}">
                                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                        </svg>
                                        <!--{$_('database.lang_column.delete')} -->
                                      </button>
                                    </div>
                                  </div>
                                {/if}
                              {:else}
                                <!-- La traduzione non esiste, mostra pulsanti: Traduci, Aggiungi da chiave e Modifica manuale -->
                                {#if editingCell.id === record.id && editingCell.column === column}
                                  <div class="flex items-center gap-2">
                                    <input
                                      class="border border-gray-300 rounded px-2 py-1 text-sm w-full"
                                      bind:value={editValue}
                                      placeholder="{$_('database.enter_value')}" />
                                    <button
                                      on:click={() => saveEdit(record.id, column)}
                                      class="bg-green-500 hover:bg-green-600 text-white text-xs font-bold py-1 px-2 rounded">
                                      {$_('database.save')}
                                    </button>
                                    <button
                                      on:click={cancelEdit}
                                      class="bg-gray-300 hover:bg-gray-400 text-gray-800 text-xs font-bold py-1 px-2 rounded">
                                      {$_('database.cancel')}
                                    </button>
                                  </div>
                                {:else}
                                  {@const action = getTranslationAction(record, column)}
                                  <div class="flex items-center justify-end gap-2">
                                    {#if action && action.type === 'translate'}
                                      <button
                                        on:click={action.action}
                                        disabled={isTranslating}
                                        class="bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 text-white text-xs font-bold py-1 px-2 rounded flex items-center gap-1"
                                        title={action.text}>
                                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                                        </svg>
                                        {action.text}
                                      </button>
                                    {/if}

                                    {#if record['key']}
                                      <button
                                        on:click={() => addFromKey(record.id, column, record['key'])}
                                        class="bg-indigo-500 hover:bg-indigo-600 text-white text-xs font-bold py-1 px-2 rounded flex items-center gap-1"
                                        title="{$_('database.add_value_from_key')}">
                                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                                        </svg>
                                        {$_('database.add_from_key')}
                                      </button>
                                    {/if}

                                    <!-- Bottone per abilitare modifica manuale inline -->
                                    <button
                                      on:click={() => startEdit(record.id, column, record[column] || '')}
                                      class="bg-yellow-400 hover:bg-yellow-500 text-white text-xs font-bold py-1 px-2 rounded"
                                      title="{$_('database.manual_edit')}">
                                      {$_('database.edit')}
                                    </button>

                                    {#if !action && !record['key']}
                                      <span class="text-gray-400">—</span>
                                    {/if}
                                  </div>
                                {/if}
                              {/if}
                            </div>
                          {:else}
                            <!-- Colonna normale (non lingua) -->
                            <div class="max-w-xs truncate" title={record[column] || ''}>
                              {#if searchTerm && record[column] && record[column].toString().toLowerCase().includes(searchTerm.toLowerCase())}
                                {@html record[column].toString().replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-1 rounded">$1</mark>')}
                              {:else}
                                {record[column] || '—'}
                              {/if}
                            </div>
                          {/if}
                        {/if}
                      </td>
                    {/each}
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

  <!-- Modal di conferma -->
  {#if showConfirmModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            {#if confirmModalType === 'error'}
              <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
            {:else if confirmModalType === 'confirm'}
              <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z"></path>
              </svg>
            {:else}
              <svg class="w-6 h-6 text-blue-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
            {/if}
            <h3 class="text-lg font-semibold text-gray-900">{confirmModalTitle}</h3>
          </div>
          <p class="text-gray-600 mb-6">{confirmModalMessage}</p>
          <div class="flex justify-end gap-3">
            {#if confirmModalType === 'confirm'}
              <button
                on:click={closeConfirmModal}
                class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
                {$_('database.cancel')}
              </button>
              <button
                on:click={confirmRemoveUnusedKeys}
                class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
                {$_('database.delete')}
              </button>
            {:else}
              <button
                on:click={closeConfirmModal}
                class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
                {$_('database.ok')}
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal di conferma eliminazione -->
  {#if showDeleteModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z"></path>
            </svg>
            <h3 class="text-lg font-semibold text-gray-900">{$_('database.delete_confirmation_title')}</h3>
          </div>
          <p class="text-gray-600 mb-6">{$_('database.delete_confirmation_message')}</p>
          <div class="flex justify-end gap-3">
            <button
              on:click={cancelDeleteRecord}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
              {$_('database.cancel')}
            </button>
            <button
              on:click={deleteRecord}
              class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
              {$_('database.delete')}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>