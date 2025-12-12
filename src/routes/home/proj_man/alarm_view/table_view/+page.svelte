<script>
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { _ } from 'svelte-i18n';

  let tableName = '';
  let projectName = ''; // Nome del progetto senza _alarms
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
  let isUploadingToMachine = false;
  // Inline edit state for manual editing of language cell values
  let editingCell = { id: null, column: null };
  let editValue = '';
   // Inline edit for alarm table cells
  let inlineEditCell = { recordId: null, column: null };
  let inlineEditValue = '';
  // Multi-select state
  let selectedRecords = new Set();
  let showBulkDeleteModal = false;
  let lastSelectedIndex = -1;
    // Calcola se tutti i record filtrati sono selezionati
    $: allSelected = filteredRecords.length > 0 && filteredRecords.every(r => selectedRecords.has(r.id));
    $: someSelected = selectedRecords.size > 0;

    function toggleSelectAll() {
      if (allSelected) {
        filteredRecords.forEach(r => selectedRecords.delete(r.id));
      } else {
        filteredRecords.forEach(r => selectedRecords.add(r.id));
      }
      selectedRecords = new Set(selectedRecords); // Trigger reattività
      lastSelectedIndex = -1;
    }

    function toggleSelectRecord(recordId, index, event) {
      // Shift+Click per selezione range
      if (event && event.shiftKey && lastSelectedIndex !== -1) {
        const start = Math.min(lastSelectedIndex, index);
        const end = Math.max(lastSelectedIndex, index);
        for (let i = start; i <= end; i++) {
          selectedRecords.add(filteredRecords[i].id);
        }
        selectedRecords = new Set(selectedRecords);
      } else {
        if (selectedRecords.has(recordId)) {
          selectedRecords.delete(recordId);
        } else {
          selectedRecords.add(recordId);
        }
        selectedRecords = new Set(selectedRecords);
        lastSelectedIndex = index;
      }
    }

    function clearSelection() {
      selectedRecords = new Set();
      lastSelectedIndex = -1;
    }

    function openBulkDeleteModal() {
      showBulkDeleteModal = true;
    }

    function closeBulkDeleteModal() {
      showBulkDeleteModal = false;
    }

    async function confirmBulkDelete() {
      try {
        const idsToDelete = Array.from(selectedRecords);
        for (const id of idsToDelete) {
          await invoke('delete_alarm', {
            tableName: projectName,
            alarmId: id.toString()
          });
        }
        records = records.filter(r => !selectedRecords.has(r.id));
        selectedRecords = new Set();
        lastSelectedIndex = -1;
        showBulkDeleteModal = false;
        toastMsg = $_('database.bulk_delete_success', { values: { count: idsToDelete.length } });
        toastType = 'success';
        showToast = true;
        setTimeout(() => showToast = false, 3000);
      } catch (error) {
        console.error('Errore nell\'eliminazione multipla:', error);
        toastMsg = $_('database.bulk_delete_error') + ' ' + error;
        toastType = 'error';
        showToast = true;
      }
    }
  // Per nuova area
  let newAreaValue = '';
  // Delete confirmation state
  let showDeleteModal = false;
  let recordToDelete = null;
  // Files popup state
  let showFilesPopup = false;
  let popupFiles = [];
  // Page filter state
  let selectedPageFilter = 'all';
  let availablePages = [];

  // NEW: Alarm CRUD state
  let showAlarmModal = false;
  let alarmModalMode = 'add'; // 'add' or 'edit'
  let currentAlarm = {
    id: null,
    alarm_name: '',
    device: '',
    variable: '',
    area: '',
    enabled: 'True',
    threshold_name: '',
    threshold_title: '',
    threshold_help: '',
    severity: '0',
    condition: '0',
    threshold_value: '0',
    sec_delay: '0',
    support_ack: 'True',
    support_reset: 'True',
    log: 'True',
    print: 'False',
    beep_enabled: 'False',
    back_color: '4294967295',
    text_color: '4294967295',
    blink_back_color: '4294967295',
    blink_text_color: '4294967295'
  };

  // Variable selection modal state
  let showVariableModal = false;
  let availableVariables = [];
  let filteredVariables = [];
  let variableSearchTerm = '';
  
  // Translation key selection modal state
  let showTranslationKeyModal = false;
  let availableTranslationKeys = [];
  let filteredTranslationKeys = [];
  let translationKeySearchTerm = '';
  let keyFieldToUpdate = ''; // 'threshold_title' or 'threshold_help'
  let translationLanguages = []; // Available languages for translation preview
  let selectedTranslationLanguage = ''; // Currently selected language for preview

  onMount(async () => {
    const urlParams = $page.url.searchParams;
    tableName = urlParams.get('table') || '';
    // Estrai il nome del progetto rimuovendo '_alarms'
    projectName = tableName.replace(/_alarms$/, '');
    console.log('Table name:', tableName);
    console.log('Project name:', projectName);

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

  // Upload alarms to machine directory
  async function uploadToMachine() {
    isUploadingToMachine = true;
    try {
      // Get machine path from settings
      const machinePath = await invoke('get_setting', { key: 'machine_path' });
      
      if (!machinePath) {
        toastMsg = $_('alarms.machine_path_not_configured');
        toastType = 'error';
        showToast = true;
        setTimeout(() => { showToast = false; }, 4000);
        isUploadingToMachine = false;
        return;
      }
      
      // Export to machine
      const result = await invoke('export_alarms_to_machine', { 
        tableName: projectName, 
        machinePath: machinePath 
      });
      
      toastMsg = $_('alarms.upload_to_machine_success') + ' ' + result;
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
    } catch (e) {
      console.error('Error uploading to machine:', e);
      toastMsg = $_('alarms.upload_to_machine_error') + ' ' + e;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
    }
    isUploadingToMachine = false;
  }

  // Filtra le colonne da mostrare (nascondi quelle interne/tecniche) e riordina
  function getVisibleColumns(columns) {
    const hiddenColumns = [
      'created_at', 'updated_at', 'timestamp', 'created', 'updated',
      'back_color', 'text_color', 'blink_back_color', 'blink_text_color'
    ];
    const filteredColumns = columns.filter(col => !hiddenColumns.includes(col.toLowerCase()));
    
    // Riordina le colonne per allarmi
    const orderedColumns = [];
    
    // Prima le colonne prioritarie nell'ordine desiderato per allarmi
    const priorityOrder = [
      'id',
      'alarm_name',
      'enabled',
      'condition',
      'threshold_value',
      'variable',
      'area',
      'threshold_name',
      'threshold_title',
      'threshold_help',
      // Colonne boolean dopo le principali
      'support_ack',
      'support_reset',
      'log',
      'print',
      'beep_enabled',
      'blink_on_new_alarm'
    ];
    
    priorityOrder.forEach(col => {
      if (filteredColumns.includes(col)) orderedColumns.push(col);
    });
    
    // Poi tutte le altre colonne (escludendo quelle già aggiunte)
    filteredColumns.forEach(col => {
      if (!priorityOrder.includes(col)) {
        orderedColumns.push(col);
      }
    });
    
    return orderedColumns;
  }

  // Filtra i record in base al termine di ricerca e al filtro pagina
  function filterRecords(records, searchTerm, pageFilter) {
    let filtered = records;
    
    // Filtro per pagina
    if (pageFilter && pageFilter !== 'all') {
      filtered = filtered.filter(record => {
        const keyFiles = parseKeyFiles(record['key_files'] || '');
        return keyFiles.some(fileName => {
          const fileNameWithoutExt = fileName.includes('.') ? fileName.substring(0, fileName.lastIndexOf('.')) : fileName;
          return fileNameWithoutExt === pageFilter;
        });
      });
    }
    
    // Filtro per termine di ricerca
    if (searchTerm && searchTerm.trim()) {
      const term = searchTerm.toLowerCase();
      filtered = filtered.filter(record => {
        return getVisibleColumns(columns).some(column => {
          const value = record[column];
          return value && value.toString().toLowerCase().includes(term);
        });
      });
    }
    
    return filtered;
  }

  // Estrae le pagine disponibili dai record
  function extractAvailablePages(records) {
    const pagesSet = new Set(['all']); // Sempre includi "Tutte"
    
    records.forEach(record => {
      const keyFiles = parseKeyFiles(record['key_files'] || '');
      keyFiles.forEach(fileName => {
        const fileNameWithoutExt = fileName.includes('.') ? fileName.substring(0, fileName.lastIndexOf('.')) : fileName;
        if (fileNameWithoutExt) {
          pagesSet.add(fileNameWithoutExt);
        }
      });
    });
    
    return Array.from(pagesSet).sort((a, b) => {
      if (a === 'all') return -1;
      if (b === 'all') return 1;
      return a.localeCompare(b);
    });
  }

  // Reattiva per aggiornare i record filtrati e le pagine disponibili
  $: availablePages = extractAvailablePages(records);
  $: filteredRecords = filterRecords(records, searchTerm, selectedPageFilter);
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
    return visibleColumns.filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'key_files' && col !== 'image_path' && col !== 'order').includes(column);
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
      .filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'key_files' && col !== 'image_path' && col !== 'order')
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

  // Funzione per verificare se una colonna ha valori vuoti da tradurre
  function hasEmptyValues(column) {
    if (!isLanguageColumn(column)) return false;
    if (!filteredRecords || filteredRecords.length === 0) return false;
    
    // Conta solo i record che hanno una chiave e sono vuoti nella colonna specifica
    const emptyCount = filteredRecords.filter(record => 
      (!record[column] || !record[column].trim()) && 
      record['key'] && record['key'].trim()
    ).length;
    
    return emptyCount > 0;
  }

  // Funzione per contare quanti valori vuoti ha una colonna
  function countEmptyValues(column) {
    if (!isLanguageColumn(column)) return 0;
    if (!filteredRecords || filteredRecords.length === 0) return 0;
    
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
            .filter(col => col !== 'id' && col !== 'key' && col !== 'keys_project' && col !== 'key_files' && col !== 'image_path' && col !== 'order')
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

  // Funzione per parsare e visualizzare i file come tag
  function parseKeyFiles(keyFilesJson) {
    if (!keyFilesJson || keyFilesJson.trim() === '') return [];
    
    try {
      const files = JSON.parse(keyFilesJson);
      if (Array.isArray(files)) {
        // Estrae solo il nome del file dal percorso completo
        return files.map(filePath => {
          const fileName = filePath.split(/[/\\]/).pop(); // Funziona sia per / che per \
          return fileName || filePath;
        });
      }
      return [];
    } catch (error) {
      console.warn('Errore nel parsing di key_files:', error, keyFilesJson);
      return [];
    }
  }

  // Funzione per aprire il popup con i file
  function openFilesPopup(keyFilesJson) {
    popupFiles = parseKeyFiles(keyFilesJson);
    showFilesPopup = true;
  }

  // Funzione per chiudere il popup
  function closeFilesPopup() {
    showFilesPopup = false;
    popupFiles = [];
  }

  // ==================== COLOR CONVERSION FUNCTIONS ====================
  
  // Convert ARGB integer (PremiumHMI format) to HEX color (#RRGGBB)
  // PremiumHMI uses ARGB format: Alpha(8bit) Red(8bit) Green(8bit) Blue(8bit)
  // Example: 4278190335 (0xFF0000FF) = Blue (A=FF, R=00, G=00, B=FF -> #0000FF)
  // Example: 4294901760 (0xFFFF0000) = Red (A=FF, R=FF, G=00, B=00 -> #FF0000)
  // Special: 4294967295 (0xFFFFFFFF) = Default/System color (shown as red in PremiumHMI)
  function argbToHex(argb) {
    if (!argb || argb === '4294967295' || argb === '') return '#FFFFFF';
    const num = parseInt(argb);
    if (isNaN(num)) return '#FFFFFF';
    
    // Extract components from ARGB format
    const r = (num >> 16) & 0xFF;   // Red is in byte 2
    const g = (num >> 8) & 0xFF;    // Green is in byte 1
    const b = num & 0xFF;           // Blue is in the lowest byte
    // Alpha is in byte 3 (ignored for display)
    
    return '#' + [r, g, b].map(x => x.toString(16).padStart(2, '0')).join('').toUpperCase();
  }
  
  // Convert HEX color (#RRGGBB) to ARGB integer (PremiumHMI format)
  function hexToArgb(hex) {
    if (!hex || hex === '#FFFFFF' || hex === '#ffffff') return '4294967295';
    
    // Remove # if present
    hex = hex.replace('#', '');
    
    // Parse RGB values
    const r = parseInt(hex.substring(0, 2), 16);
    const g = parseInt(hex.substring(2, 4), 16);
    const b = parseInt(hex.substring(4, 6), 16);
    
    // Create ARGB with full alpha (255)
    // PremiumHMI format: A(byte3) R(byte2) G(byte1) B(byte0)
    const argb = (255 << 24) | (r << 16) | (g << 8) | b;
    
    // Convert to unsigned 32-bit integer
    return (argb >>> 0).toString();
  }
  
  // Check if color is default (white/transparent)
  function isDefaultColor(argb) {
    return !argb || argb === '4294967295' || argb === '';
  }

  // ==================== ALARM TYPE FUNCTIONS ====================
  
  // Determine alarm type based on colors
  // Messaggio Giallo: TextColor=65535 (Yellow)
  // Messaggio Verde: TextColor=65280 (Green)
  // Messaggio Rosso: TextColor=255 (Red) with BackColor=4294967295 (White)
  function getAlarmType(record) {
    const textColor = record.text_color;
    const backColor = record.back_color;
    
    // Messaggio Giallo: TextColor=65535
    if (textColor === '65535') {
      return 'yellow';
    }
    // Messaggio Verde: TextColor=65280
    if (textColor === '65280') {
      return 'green';
    }
    // Messaggio Rosso: TextColor=255 (Red)
    if (textColor === '255') {
      return 'red';
    }
    // Default to red for any unrecognized type
    return 'red';
  }

  // Apply alarm type preset to a record
  async function setAlarmType(record, type) {
    const updates = {};
    
    if (type === 'yellow') {
      // MESSAGGIO GIALLO
      updates.back_color = '0';
      updates.text_color = '65535';
      updates.blink_back_color = '13922560';
      updates.blink_text_color = '0';
      updates.print = 'True';
      updates.log = 'False';
      updates.blink_on_new_alarm = 'True';
      updates.support_ack = 'False';
      updates.support_reset = 'False';
      updates.beep_enabled = 'True';
    } else if (type === 'green') {
      // MESSAGGIO VERDE
      updates.back_color = '0';
      updates.text_color = '65280';
      updates.blink_back_color = '13922560';
      updates.blink_text_color = '0';
      updates.print = 'True';
      updates.log = 'True';
      updates.blink_on_new_alarm = 'False';
      updates.support_ack = 'False';
      updates.support_reset = 'False';
      updates.beep_enabled = 'False';
    } else {
      // MESSAGGIO ROSSO (default)
      updates.back_color = '4294967295';
      updates.text_color = '255';
      updates.blink_back_color = '4294967295';
      updates.blink_text_color = '4294967295';
      updates.print = 'True';
      updates.log = 'True';
      updates.blink_on_new_alarm = 'True';
      updates.support_ack = 'True';
      updates.support_reset = 'True';
      updates.beep_enabled = 'True';
    }
    
    // Save to database
    try {
      await invoke('update_alarm', {
        tableName: projectName,
        alarmId: record.id.toString(),
        updates: updates
      });
      
      // Update local records
      const recordIndex = records.findIndex(r => r.id === record.id);
      if (recordIndex !== -1) {
        Object.assign(records[recordIndex], updates);
        records = [...records]; // Trigger reactivity
      }
      
      toastMsg = $_('alarms.field_updated');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
    } catch (error) {
      console.error('Error updating alarm type:', error);
      toastMsg = $_('alarms.error_updating') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }

  // ==================== ALARM CRUD FUNCTIONS ====================

  // Reset alarm form to defaults
  function resetAlarmForm() {
    currentAlarm = {
      id: null,
      alarm_name: '',
      device: '',
      variable: '',
      area: '',
      enabled: 'True',
      threshold_name: '',
      threshold_title: '',
      threshold_help: '',
      severity: '0',
      condition: '0',
      threshold_value: '0',
      sec_delay: '0',
      support_ack: 'True',
      support_reset: 'True',
      log: 'True',
      print: 'False',
      beep_enabled: 'False',
      blink_on_new_alarm: 'True',
      back_color: '4294967295',
      text_color: '255',
      blink_back_color: '4294967295',
      blink_text_color: '4294967295'
    };
  }

  // Open modal to add new alarm
  function openAddAlarmModal() {
    resetAlarmForm();
    alarmModalMode = 'add';
    showAlarmModal = true;
  }

  // Open modal to edit existing alarm
  function openEditAlarmModal(record) {
    currentAlarm = { ...record };
    alarmModalMode = 'edit';
    showAlarmModal = true;
  }

  // Close alarm modal
  function closeAlarmModal() {
    showAlarmModal = false;
    resetAlarmForm();
  }

  // Toggle boolean value directly in table
  async function toggleBooleanField(record, field) {
    const currentValue = record[field];
    const newValue = (currentValue === 'True' || currentValue === 'true' || currentValue === '1') ? 'False' : 'True';
    
    try {
      const updates = {};
      updates[field] = newValue;
      
      await invoke('update_alarm', {
        tableName: projectName,
        alarmId: record.id.toString(),
        updates: updates
      });
      
      // Update local record
      const recordIndex = records.findIndex(r => r.id === record.id);
      if (recordIndex !== -1) {
        records[recordIndex][field] = newValue;
        records = [...records]; // Trigger reactivity
      }
      
      toastMsg = $_('alarms.field_updated');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
      
    } catch (error) {
      console.error('Error toggling field:', error);
      toastMsg = $_('alarms.error_updating') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }

  // ==================== INLINE EDIT FUNCTIONS ====================
  
  // Start inline editing for a cell
  function startInlineEdit(record, column) {
    inlineEditCell = { recordId: record.id, column: column };
    inlineEditValue = record[column] || '';
  }
  
  // Cancel inline editing
  function cancelInlineEdit() {
    inlineEditCell = { recordId: null, column: null };
    inlineEditValue = '';
  }
  
  // Save inline edit
  async function saveInlineEdit(record, forcedValue = undefined) {
    if (inlineEditCell.recordId === null) return;
    try {
      const updates = {};
      let value = forcedValue !== undefined ? forcedValue : inlineEditValue;
      // Forza sempre stringa per threshold_value e sec_delay
      if (inlineEditCell.column === 'threshold_value' || inlineEditCell.column === 'sec_delay') {
        updates[inlineEditCell.column] = String(value);
      } else {
        updates[inlineEditCell.column] = value;
      }
      await invoke('update_alarm', {
        tableName: projectName,
        alarmId: record.id.toString(),
        updates: updates
      });
      // Update local record
      const recordIndex = records.findIndex(r => r.id === record.id);
      if (recordIndex !== -1) {
        records[recordIndex][inlineEditCell.column] = value;
        records = [...records]; // Trigger reactivity
      }
      toastMsg = $_('alarms.field_updated');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
      cancelInlineEdit();
    } catch (error) {
      console.error('Error saving inline edit:', error);
      toastMsg = $_('alarms.error_updating') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }
  
  // Update field directly (for dropdowns)
  async function updateFieldDirect(record, column, value) {
    try {
      const updates = {};
      updates[column] = value;
      
      await invoke('update_alarm', {
        tableName: projectName,
        alarmId: record.id.toString(),
        updates: updates
      });
      
      // Update local record
      const recordIndex = records.findIndex(r => r.id === record.id);
      if (recordIndex !== -1) {
        records[recordIndex][column] = value;
        records = [...records]; // Trigger reactivity
      }
      
      toastMsg = $_('alarms.field_updated');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 2000);
    } catch (error) {
      console.error('Error updating field:', error);
      toastMsg = $_('alarms.error_updating') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
    }
  }
  
  // Open variable selection for inline edit
  function openInlineVariableSelect(record, column) {
    inlineEditCell = { recordId: record.id, column: column };
    loadAvailableVariables();
    showVariableModal = true;
  }
  
  // Open translation key selection for inline edit
  function openInlineTranslationKeySelect(record, column) {
    inlineEditCell = { recordId: record.id, column: column };
    keyFieldToUpdate = column;
    loadAvailableTranslationKeys();
    showTranslationKeyModal = true;
  }

  // Save alarm (create or update)
  async function saveAlarm() {
    try {
      if (alarmModalMode === 'add') {
        // Create new alarm
        const alarmData = { ...currentAlarm };
        delete alarmData.id; // Remove id for new records
        
        await invoke('add_alarm', {
          tableName: projectName,
          alarmData: alarmData
        });
        
        toastMsg = $_('alarms.alarm_created');
        toastType = 'success';
      } else {
        // Update existing alarm
        const updates = { ...currentAlarm };
        const alarmId = updates.id;
        delete updates.id;
        
        await invoke('update_alarm', {
          tableName: projectName,
          alarmId: alarmId,
          updates: updates
        });
        
        toastMsg = $_('alarms.alarm_updated');
        toastType = 'success';
      }
      
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      closeAlarmModal();
      await loadDatabaseData();
      
    } catch (error) {
      console.error('Error saving alarm:', error);
      toastMsg = (alarmModalMode === 'add' ? $_('alarms.error_creating') : $_('alarms.error_updating')) + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
    }
  }

  // Open delete confirmation for alarm
  function openDeleteAlarmConfirm(record) {
    recordToDelete = record.id;
    showDeleteModal = true;
  }

  // Delete alarm
  async function deleteAlarmRecord() {
    if (!recordToDelete) return;
    
    try {
      await invoke('delete_alarm', {
        tableName: projectName,
        alarmId: recordToDelete.toString()
      });
      
      toastMsg = $_('alarms.alarm_deleted');
      toastType = 'success';
      showToast = true;
      setTimeout(() => { showToast = false; }, 3000);
      
      showDeleteModal = false;
      recordToDelete = null;
      await loadDatabaseData();
      
    } catch (error) {
      console.error('Error deleting alarm:', error);
      toastMsg = $_('alarms.error_deleting') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
      showDeleteModal = false;
      recordToDelete = null;
    }
  }

  // ==================== VARIABLE SELECTION ====================

  // Load available variables (shared between modal and inline)
  async function loadAvailableVariables() {
    try {
      availableVariables = await invoke('get_variables', { tableName: projectName });
      filteredVariables = [...availableVariables];
      variableSearchTerm = '';
    } catch (error) {
      console.error('Error loading variables:', error);
      toastMsg = $_('alarms.error_loading_variables') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
    }
  }

  // Open variable selection modal
  async function openVariableModal() {
    await loadAvailableVariables();
    showVariableModal = true;
  }

  // Filter variables based on search
  function filterVariables() {
    if (!variableSearchTerm.trim()) {
      filteredVariables = [...availableVariables];
    } else {
      const term = variableSearchTerm.toLowerCase();
      filteredVariables = availableVariables.filter(v => 
        v.name?.toLowerCase().includes(term) ||
        v.description?.toLowerCase().includes(term) ||
        v.var_group?.toLowerCase().includes(term)
      );
    }
  }

  // React to variable search term changes
  $: if (availableVariables.length > 0) {
    filterVariables();
  }

  // Select a variable
  async function selectVariable(variable) {
    // Check if we're in inline edit mode
    if (inlineEditCell.recordId !== null && inlineEditCell.column === 'variable') {
      // Update the record directly
      const record = records.find(r => r.id === inlineEditCell.recordId);
      if (record) {
        const updates = {
          variable: variable.name,
          device: variable.device || '',
          area: variable.area_type || ''
        };
        
        try {
          await invoke('update_alarm', {
            tableName: projectName,
            alarmId: record.id.toString(),
            updates: updates
          });
          
          // Update local record
          const recordIndex = records.findIndex(r => r.id === record.id);
          if (recordIndex !== -1) {
            Object.assign(records[recordIndex], updates);
            records = [...records];
          }
          
          toastMsg = $_('alarms.field_updated');
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 2000);
        } catch (error) {
          console.error('Error updating variable:', error);
          toastMsg = $_('alarms.error_updating') + ' ' + error;
          toastType = 'error';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        }
      }
      inlineEditCell = { recordId: null, column: null };
    } else {
      // Modal mode - update currentAlarm
      currentAlarm.variable = variable.name;
      currentAlarm.device = variable.device || '';
      currentAlarm.area = variable.area_type || '';
    }
    showVariableModal = false;
  }

  // Close variable modal
  function closeVariableModal() {
    showVariableModal = false;
    availableVariables = [];
    filteredVariables = [];
    variableSearchTerm = '';
    // Clear inline edit if was triggered from table
    if (inlineEditCell.column === 'variable') {
      inlineEditCell = { recordId: null, column: null };
    }
  }

  // ==================== TRANSLATION KEY SELECTION ====================

  // Load available translation keys (shared between modal and inline)
  async function loadAvailableTranslationKeys() {
    try {
      availableTranslationKeys = await invoke('get_translation_keys', { tableName: projectName });
      filteredTranslationKeys = [...availableTranslationKeys];
      translationKeySearchTerm = '';
      
      // Extract available languages from the first key (all keys have the same languages)
      if (availableTranslationKeys.length > 0 && availableTranslationKeys[0]._languages) {
        translationLanguages = availableTranslationKeys[0]._languages.split(',').filter(l => l);
        selectedTranslationLanguage = translationLanguages.length > 0 ? translationLanguages[0] : '';
      } else {
        translationLanguages = [];
        selectedTranslationLanguage = '';
      }
    } catch (error) {
      console.error('Error loading translation keys:', error);
      toastMsg = $_('alarms.error_loading_keys') + ' ' + error;
      toastType = 'error';
      showToast = true;
      setTimeout(() => { showToast = false; }, 5000);
    }
  }

  // Open translation key selection modal
  async function openTranslationKeyModal(field) {
    keyFieldToUpdate = field;
    await loadAvailableTranslationKeys();
    showTranslationKeyModal = true;
  }

  // Filter translation keys based on search
  function filterTranslationKeys() {
    if (!translationKeySearchTerm.trim()) {
      filteredTranslationKeys = [...availableTranslationKeys];
    } else {
      const term = translationKeySearchTerm.toLowerCase();
      filteredTranslationKeys = availableTranslationKeys.filter(k => {
        // Search in key
        if (k.key?.toLowerCase().includes(term)) return true;
        // Search in all language translations
        for (const lang of translationLanguages) {
          if (k[lang]?.toLowerCase().includes(term)) return true;
        }
        return false;
      });
    }
  }

  // React to translation key search term changes
  $: if (availableTranslationKeys.length > 0) {
    filterTranslationKeys();
  }

  // Select a translation key
  async function selectTranslationKey(keyObj) {
    // Check if we're in inline edit mode
    if (inlineEditCell.recordId !== null && (inlineEditCell.column === 'threshold_title' || inlineEditCell.column === 'threshold_help')) {
      // Update the record directly
      const record = records.find(r => r.id === inlineEditCell.recordId);
      if (record) {
        const updates = {};
        updates[inlineEditCell.column] = keyObj.key;
        
        try {
          await invoke('update_alarm', {
            tableName: projectName,
            alarmId: record.id.toString(),
            updates: updates
          });
          
          // Update local record
          const recordIndex = records.findIndex(r => r.id === record.id);
          if (recordIndex !== -1) {
            records[recordIndex][inlineEditCell.column] = keyObj.key;
            records = [...records];
          }
          
          toastMsg = $_('alarms.field_updated');
          toastType = 'success';
          showToast = true;
          setTimeout(() => { showToast = false; }, 2000);
        } catch (error) {
          console.error('Error updating translation key:', error);
          toastMsg = $_('alarms.error_updating') + ' ' + error;
          toastType = 'error';
          showToast = true;
          setTimeout(() => { showToast = false; }, 3000);
        }
      }
      inlineEditCell = { recordId: null, column: null };
    } else {
      // Modal mode - update currentAlarm
      if (keyFieldToUpdate === 'threshold_title') {
        currentAlarm.threshold_title = keyObj.key;
      } else if (keyFieldToUpdate === 'threshold_help') {
        currentAlarm.threshold_help = keyObj.key;
      }
    }
    showTranslationKeyModal = false;
  }

  // Close translation key modal
  function closeTranslationKeyModal() {
    showTranslationKeyModal = false;
    availableTranslationKeys = [];
    filteredTranslationKeys = [];
    translationKeySearchTerm = '';
    keyFieldToUpdate = '';
    // Clear inline edit if was triggered from table
    if (inlineEditCell.column === 'threshold_title' || inlineEditCell.column === 'threshold_help') {
      inlineEditCell = { recordId: null, column: null };
    }
  }

  // Helper to get variable type label
  function getVariableTypeLabel(type) {
    const types = {
      '0': 'Bool', '1': 'Byte', '2': 'Char', '3': 'Int', '4': 'UInt',
      '5': 'DInt', '6': 'UDInt', '7': 'Real', '8': 'LReal', '9': 'String',
      '10': 'WString', '11': 'Struct', '12': 'Array'
    };
    return types[type] || type;
  }

  // Helper to get area type label
  function getAreaTypeLabel(area) {
    const areas = { '0': 'Input', '1': 'Output', '2': 'Flag', '3': 'Memory' };
    return areas[area] || area;
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
          class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={openAddAlarmModal}
          disabled={loading}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
          </svg>
          {$_('alarms.add_alarm')}
        </button>
        <button 
          class="bg-orange-500 hover:bg-orange-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2" 
          on:click={uploadToMachine}
          disabled={loading || isUploadingToMachine}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"></path>
          </svg>
          {isUploadingToMachine ? $_('database.loading') : $_('alarms.upload_to_machine')}
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
      <div class="fixed top-24 left-5 right-5 z-20 bg-white/95 backdrop-blur-md rounded-lg border border-gray-300/50  p-4">
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
            
            <!-- Search summary: appear between search input and page filter -->
            {#if searchTerm}
              <div class="ml-3 mt-0 text-sm text-gray-600 bg-yellow-50 rounded-lg p-1">
                <svg class="inline w-4 h-4 mr-1 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
                {$_('database.search_results', { values: { count: filteredRecords.length, term: searchTerm } })}
              </div>
            {/if}

            <!-- Filtro per pagina -->
            <div class="relative">
              <select
                bind:value={selectedPageFilter}
                class="appearance-none bg-white border border-gray-300 rounded-md px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500">
                {#each availablePages as page}
                  <option value={page}>
                    {page === 'all' ? $_('database.all_pages') : page}
                  </option>
                {/each}
              </select>
              <div class="absolute inset-y-0 right-0 flex items-center px-2 pointer-events-none">
                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                </svg>
              </div>
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
          
          
        </div>

        <!-- Contenitore per la tabella -->
        <div class="w-full pb-18" style="margin-top: 80px;">
          
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
                      <!-- Checkbox header per seleziona tutto -->
                      <th class="py-3 px-3 text-center bg-gray-50/80 w-10">
                        <div class="flex flex-col items-center gap-1">
                          <input
                            type="checkbox"
                            checked={allSelected}
                            on:change={toggleSelectAll}
                            class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                            title={allSelected ? $_('database.deselect_all') : $_('database.select_all')} />
                        </div>
                      </th>
                      {#each visibleColumns as column, columnIndex}
                        <th class="py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider {columnIndex % 2 === 0 ? 'bg-gray-50/80' : 'bg-blue-100/60'} px-4 {column === 'threshold_value' ? 'min-w-[120px]' : ''} {column === 'area' ? 'min-w-[140px]' : ''}">
                          <span>
                            {column === 'id' ? 'ID' : 
                             column === 'alarm_name' ? $_('alarms.alarm_name') :
                             column === 'device' ? $_('alarms.alarm_device') :
                             column === 'variable' ? $_('alarms.alarm_variable') :
                             column === 'area' ? $_('alarms.alarm_area') :
                             column === 'enabled' ? $_('alarms.alarm_enabled') :
                             column === 'threshold_name' ? $_('alarms.threshold_name') :
                             column === 'threshold_title' ? $_('alarms.threshold_title') :
                             column === 'threshold_help' ? $_('alarms.threshold_help') :
                             column === 'severity' ? $_('alarms.severity') :
                             column === 'condition' ? $_('alarms.condition') :
                             column === 'threshold_value' ? $_('alarms.threshold_value') :
                             column === 'sec_delay' ? $_('alarms.sec_delay') :
                             column === 'support_ack' ? $_('alarms.support_ack') :
                             column === 'support_reset' ? $_('alarms.support_reset') :
                             column === 'log' ? $_('alarms.log') :
                             column === 'print' ? $_('alarms.print') :
                             column === 'beep_enabled' ? $_('alarms.beep_enabled') :
                             column === 'blink_on_new_alarm' ? 'Lampeggia' :
                             column === 'back_color' ? $_('alarms.back_color') :
                             column === 'text_color' ? $_('alarms.text_color') :
                             column === 'blink_back_color' ? $_('alarms.blink_back_color') :
                             column === 'blink_text_color' ? $_('alarms.blink_text_color') :
                             column === 'source_file' ? 'File' :
                             column}
                          </span>
                        </th>
                      {/each}
                      <th class="py-3 px-4 text-left text-xs font-medium text-gray-500 uppercase tracking-wider bg-gray-50/80 sticky right-0">
                        {$_('variables.column_actions')}
                      </th>
                    </tr>
                  </thead>
                  <tbody class="bg-white/60 divide-y divide-gray-200">
                {#each filteredRecords as record, index}
                  <tr class="group hover:bg-gray-50/80 transition-colors {selectedRecords.has(record.id) ? 'bg-blue-50/60' : ''}">
                    <!-- Checkbox per selezione riga (con supporto Shift+Click) -->
                    <td class="py-3 px-3 text-center {selectedRecords.has(record.id) ? 'bg-blue-100/60' : 'bg-white/60'} w-10 select-none">
                      <input
                        type="checkbox"
                        checked={selectedRecords.has(record.id)}
                        on:click={(e) => toggleSelectRecord(record.id, index, e)}
                        class="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 cursor-pointer"
                        title={$_('database.shift_click_hint')} />
                    </td>
                    {#each visibleColumns as column, columnIndex}
                      <td class="py-3 px-4 text-sm text-gray-900 {columnIndex % 2 === 0 ? 'bg-white/60' : 'bg-blue-50/40'} whitespace-nowrap">
                        {#if column === 'enabled' || column === 'support_ack' || column === 'support_reset' || column === 'log' || column === 'print' || column === 'beep_enabled' || column === 'blink_on_new_alarm'}
                          <!-- Boolean columns with clickable colored badges -->
                          <button
                            on:click={() => toggleBooleanField(record, column)}
                            class="cursor-pointer hover:scale-105 transition-transform focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-blue-500 rounded-full"
                            title="{$_('alarms.click_to_toggle')}">
                            {#if record[column] === 'True' || record[column] === 'true' || record[column] === '1'}
                              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800 hover:bg-green-200">
                                <svg class="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                                  <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
                                </svg>
                                {$_('alarms.true')}
                              </span>
                            {:else}
                              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-600 hover:bg-gray-200">
                                <svg class="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                                  <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/>
                                </svg>
                                {$_('alarms.false')}
                              </span>
                            {/if}
                          </button>
                        {:else if column === 'severity'}
                          <!-- Severity with inline dropdown -->
                          <select
                            value={record[column] || '0'}
                            on:change={(e) => updateFieldDirect(record, column, e.target.value)}
                            class="text-xs border border-gray-300 rounded px-2 py-1 bg-white hover:border-blue-400 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 cursor-pointer {
                              parseInt(record[column]) >= 3 ? 'text-red-700 bg-red-50' :
                              parseInt(record[column]) === 2 ? 'text-orange-700 bg-orange-50' :
                              parseInt(record[column]) === 1 ? 'text-yellow-700 bg-yellow-50' :
                              'text-blue-700 bg-blue-50'
                            }">
                            <option value="0">{$_('alarms.severity_low')}</option>
                            <option value="1">{$_('alarms.severity_medium')}</option>
                            <option value="2">{$_('alarms.severity_high')}</option>
                            <option value="3">{$_('alarms.severity_critical')}</option>
                          </select>
                        {:else if column === 'condition'}
                          <!-- Condition with inline dropdown -->
                          <select
                            value={record[column] || '0'}
                            on:change={(e) => updateFieldDirect(record, column, e.target.value)}
                            class="text-xs border border-gray-300 rounded px-2 py-1 bg-white hover:border-blue-400 focus:ring-2 focus:ring-blue-500 focus:border-blue-500 cursor-pointer">
                            <option value="0">&gt;=</option>
                            <option value="1">&lt;=</option>
                            <option value="2">==</option>
                            <option value="3">Inc val</option>
                            <option value="4">Dec val</option>
                            <option value="5">!=</option>
                          </select>
                        
                        {:else if column === 'variable'}
                          <button
                            on:click={() => openInlineVariableSelect(record, column)}
                            class="text-left w-full max-w-xs truncate hover:bg-blue-50 px-1 py-0.5 rounded cursor-pointer group flex items-center"
                            title="{$_('alarms.click_to_select')}">
                            <span class="group-hover:text-blue-600 truncate">{record[column] || '—'}</span>
                            <svg class="w-3 h-3 ml-1 text-gray-400 group-hover:text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                            </svg>
                          </button>

                        {:else if column === 'alarm_name' || column === 'threshold_name' || column === 'threshold_value' || column === 'sec_delay'}
                          <!-- Text fields with inline edit -->
                          {#if inlineEditCell.recordId === record.id && inlineEditCell.column === column}
                            <div class="flex items-center gap-1">
                              <input
                                type={column === 'threshold_value' || column === 'sec_delay' ? 'number' : 'text'}
                                bind:value={inlineEditValue}
                                on:keydown={(e) => {
                                  if (e.key === 'Enter') saveInlineEdit(record);
                                  if (e.key === 'Escape') cancelInlineEdit();
                                }}
                                class="text-xs border border-blue-400 rounded px-2 py-1 w-full focus:ring-2 focus:ring-blue-500"
                                autofocus />
                              <button on:click={() => saveInlineEdit(record)} class="text-green-600 hover:text-green-800" title="Salva">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                </svg>
                              </button>
                              <button on:click={cancelInlineEdit} class="text-red-600 hover:text-red-800" title="Annulla">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                </svg>
                              </button>
                            </div>
                          {:else}
                            <button
                              on:click={() => startInlineEdit(record, column)}
                              class="text-left w-full max-w-xs truncate hover:bg-blue-50 px-1 py-0.5 rounded cursor-text group"
                              title="{$_('alarms.click_to_edit')}">
                              <span class="group-hover:text-blue-600">{(record[column] !== undefined && record[column] !== null && record[column] !== '') ? record[column] : '—'}</span>
                              <svg class="w-3 h-3 inline ml-1 text-gray-400 opacity-0 group-hover:opacity-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                              </svg>
                            </button>
                          {/if}


                        {:else if column === 'area'}
                          <!-- Area: dropdown con aree esistenti + nuova area -->
                          {#if inlineEditCell.recordId === record.id && inlineEditCell.column === column}
                            <div class="flex items-center gap-1 ">
                              {#if inlineEditValue === '__new__'}
                                <input
                                  type="text"
                                  bind:value={newAreaValue}
                                  placeholder="Nuova area..."
                                  class="text-xs border border-blue-400 rounded px-2 py-1 w-full focus:ring-2 focus:ring-blue-500"
                                  autofocus
                                  on:keydown={(e) => {
                                    if (e.key === 'Enter') saveInlineEdit(record, newAreaValue);
                                    if (e.key === 'Escape') cancelInlineEdit();
                                  }}
                                />
                                <button on:click={() => saveInlineEdit(record, newAreaValue)} class="text-green-600 hover:text-green-800" title="Salva">
                                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                  </svg>
                                </button>
                                <button on:click={cancelInlineEdit} class="text-red-600 hover:text-red-800" title="Annulla">
                                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                  </svg>
                                </button>
                                <button on:click={() => saveInlineEdit(record, '')} class="text-gray-500 hover:text-gray-900" title="Elimina area">
                                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
                                  </svg>
                                </button>
                              {:else}
                                <select
                                  bind:value={inlineEditValue}
                                  on:change={(e) => {
                                    if (e.target.value === '__new__') {
                                      newAreaValue = '';
                                    } else {
                                      saveInlineEdit(record, e.target.value);
                                    }
                                  }}
                                  class="text-xs border border-blue-400 rounded px-2 py-1 w-full focus:ring-2 focus:ring-blue-500"
                                  autofocus
                                >
                                  {#each Array.from(new Set(filteredRecords.map(r => r.area).filter(a => a && a.trim() !== ''))).sort() as areaOption}
                                    <option value={areaOption}>{areaOption}</option>
                                  {/each}
                                  <option value="__new__">+ Nuova area...</option>
                                </select>
                                <button on:click={cancelInlineEdit} class="text-red-600 hover:text-red-800" title="Annulla">
                                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                                  </svg>
                                </button>
                                <button on:click={() => saveInlineEdit(record, '')} class="text-gray-500 hover:text-gray-900" title="Elimina area">
                                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
                                  </svg>
                                </button>
                              {/if}
                            </div>
                          {:else}
                            <button
                              on:click={() => { startInlineEdit(record, column); inlineEditValue = record[column] || ''; newAreaValue = ''; }}
                              class="text-left w-full max-w-xs truncate hover:bg-blue-50 px-1 py-0.5 rounded cursor-pointer group flex items-center"
                              title="{$_('alarms.click_to_edit')}">
                              <span class="group-hover:text-blue-600 truncate">{(record[column] !== undefined && record[column] !== null && record[column] !== '') ? record[column] : '—'}</span>
                              <svg class="w-3 h-3 ml-1 text-gray-400 group-hover:text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                              </svg>
                            </button>
                          {/if}

                        {:else if column === 'threshold_title' || column === 'threshold_help'}
                          <!-- Translation key fields with dropdown selection -->
                          <button
                            on:click={() => openInlineTranslationKeySelect(record, column)}
                            class="text-left w-full max-w-xs truncate hover:bg-blue-50 px-1 py-0.5 rounded cursor-pointer group flex items-center"
                            title="{$_('alarms.click_to_select')}">
                            <span class="group-hover:text-blue-600 truncate">{(record[column] !== undefined && record[column] !== null && record[column] !== '') ? record[column] : '—'}</span>
                            <svg class="w-3 h-3 ml-1 text-gray-400 group-hover:text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                            </svg>
                          </button>

                        {:else if column === 'back_color' || column === 'text_color' || column === 'blink_back_color' || column === 'blink_text_color'}
                          <!-- Color columns with color swatch -->
                          {@const colorValue = record[column]}
                          {@const hexColor = argbToHex(colorValue)}
                          {@const isDefault = isDefaultColor(colorValue)}
                          <div class="flex items-center gap-2">
                            <div 
                              class="w-6 h-6 rounded border-2 {isDefault ? 'border-gray-300' : 'border-gray-400'} shadow-sm"
                              style="background-color: {hexColor};"
                              title="{hexColor} ({colorValue || $_('alarms.default_color')})">
                            </div>
                            {#if !isDefault}
                              <span class="text-xs text-gray-500 font-mono">{hexColor}</span>
                            {:else}
                              <span class="text-xs text-gray-400 italic">{$_('alarms.default_color')}</span>
                            {/if}
                          </div>
                        {:else if column === 'id'}
                          <span class="text-gray-500 font-mono">{record[column]}</span>
                        {:else}
                          <!-- Default column display -->
                          <div class="max-w-xs truncate" title={record[column] || ''}>
                            {#if searchTerm && record[column] && record[column].toString().toLowerCase().includes(searchTerm.toLowerCase())}
                              {@html record[column].toString().replace(new RegExp(`(${searchTerm})`, 'gi'), '<mark class="bg-yellow-200 px-1 rounded">$1</mark>')}
                            {:else}
                              {(record[column] !== undefined && record[column] !== null && record[column] !== '') ? record[column] : '—'}
                            {/if}
                          </div>
                        {/if}
                      </td>
                    {/each}
                    <!-- Actions column -->
                    <td class="py-3 px-4 text-sm whitespace-nowrap bg-white/80 sticky right-0">
                      <div class="flex items-center gap-1">
                        <!-- ...existing code... -->
    <!-- Barra azioni selezione multipla -->
    {#if someSelected}
      <div class="fixed bottom-4 left-1/2 transform -translate-x-1/2 z-50 bg-gray-800 text-white px-6 py-3 rounded-lg shadow-xl flex items-center gap-4">
        <span class="text-sm font-medium">
          {$_('database.selected_count', { values: { count: selectedRecords.size } })}
        </span>
        <div class="h-4 w-px bg-gray-500"></div>
        <button
          on:click={openBulkDeleteModal}
          class="bg-red-500 hover:bg-red-600 text-white text-sm font-bold py-1.5 px-4 rounded flex items-center gap-2 transition-colors">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
          </svg>
          {$_('database.delete_selected')}
        </button>
        <button
          on:click={clearSelection}
          class="bg-gray-600 hover:bg-gray-500 text-white text-sm font-bold py-1.5 px-4 rounded flex items-center gap-2 transition-colors">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
          {$_('database.clear_selection')}
        </button>
      </div>
    {/if}
  <!-- Modal conferma eliminazione multipla -->
  {#if showBulkDeleteModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg max-w-md w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            <div class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-red-100">
              <svg class="h-6 w-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
              </svg>
            </div>
          </div>
          <div class="text-center">
            <h3 class="text-lg font-semibold text-gray-900 mb-2">
              {$_('database.confirm_bulk_delete')}
            </h3>
            <p class="text-sm text-gray-500">
              {$_('database.confirm_bulk_delete_message', { values: { count: selectedRecords.size } })}
            </p>
          </div>
          <div class="flex justify-center gap-3 mt-6">
            <button
              on:click={closeBulkDeleteModal}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
              {$_('database.cancel')}
            </button>
            <button
              on:click={confirmBulkDelete}
              class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
              {$_('database.delete_selected')}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
                        <!-- Messaggio Rosso -->
                        <button
                          on:click={() => setAlarmType(record, 'red')}
                          class="px-2 py-1 text-xs font-medium rounded transition-all {
                            getAlarmType(record) === 'red'
                              ? 'bg-red-500 text-white ring-2 ring-red-300'
                              : 'bg-gray-100 text-gray-600 hover:bg-red-100 hover:text-red-700'
                          }"
                          title="{$_('alarms.type_red_desc')}">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                          </svg>
                        </button>
                        <!-- Messaggio Giallo -->
                        <button
                          on:click={() => setAlarmType(record, 'yellow')}
                          class="px-2 py-1 text-xs font-medium rounded transition-all {
                            getAlarmType(record) === 'yellow'
                              ? 'bg-yellow-500 text-white ring-2 ring-yellow-300'
                              : 'bg-gray-100 text-gray-600 hover:bg-yellow-100 hover:text-yellow-700'
                          }"
                          title="{$_('alarms.type_yellow_desc')}">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
                        <!-- Messaggio Verde -->
                        <button
                          on:click={() => setAlarmType(record, 'green')}
                          class="px-2 py-1 text-xs font-medium rounded transition-all {
                            getAlarmType(record) === 'green'
                              ? 'bg-green-500 text-white ring-2 ring-green-300'
                              : 'bg-gray-100 text-gray-600 hover:bg-green-100 hover:text-green-700'
                          }"
                          title="{$_('alarms.type_green_desc')}">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
                        <span class="mx-1 border-l border-gray-300 h-4"></span>
                        <button
                          on:click={() => openEditAlarmModal(record)}
                          class="bg-yellow-500 hover:bg-yellow-600 text-white text-xs font-bold py-1 px-3 rounded flex items-center gap-1"
                          title="{$_('alarms.edit_alarm')}">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                          </svg>
                          {$_('database.edit')}
                        </button>
                        <button
                          on:click={() => openDeleteAlarmConfirm(record)}
                          class="bg-red-500 hover:bg-red-600 text-white text-xs font-bold py-1 px-3 rounded flex items-center gap-1"
                          title="{$_('alarms.delete_alarm')}">
                          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                          </svg>
                          {$_('database.delete')}
                        </button>
                      </div>
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

  <!-- Modal di conferma -->
  {#if showConfirmModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg  max-w-md w-full mx-4">
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

  <!-- Modal di conferma eliminazione allarme -->
  {#if showDeleteModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg max-w-md w-full mx-4">
        <div class="p-6">
          <div class="flex items-center mb-4">
            <svg class="w-6 h-6 text-red-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 15.5c-.77.833.192 2.5 1.732 2.5z"></path>
            </svg>
            <h3 class="text-lg font-semibold text-gray-900">{$_('alarms.confirm_delete')}</h3>
          </div>
          <p class="text-gray-600 mb-6">{$_('alarms.confirm_delete_message')}</p>
          <div class="flex justify-end gap-3">
            <button
              on:click={() => { showDeleteModal = false; recordToDelete = null; }}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
              {$_('alarms.cancel')}
            </button>
            <button
              on:click={deleteAlarmRecord}
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

  <!-- Popup per visualizzare i file -->
  {#if showFilesPopup}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-lg  max-w-lg w-full mx-4">
        <div class="p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-900 flex items-center">
              <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
              </svg>
              Files ({popupFiles.length})
            </h3>
            <button
              on:click={closeFilesPopup}
              class="text-gray-400 hover:text-gray-600 transition-colors">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          
          <div class="max-h-96 overflow-y-auto">
            <div class="flex flex-wrap gap-2">
              {#each popupFiles as fileName}
                <span class="inline-flex items-center px-3 py-2 rounded-full text-sm font-medium bg-blue-100 text-blue-800 border border-blue-200">
                  <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                  </svg>
                  {fileName.includes('.') ? fileName.substring(0, fileName.lastIndexOf('.')) : fileName}
                </span>
              {/each}
            </div>
          </div>
          
          <div class="flex justify-end mt-6">
            <button
              on:click={closeFilesPopup}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded">
              Chiudi
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal per aggiungere/modificare allarme -->
  {#if showAlarmModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
      <div class="relative bg-white rounded-xl shadow-2xl max-w-3xl w-full mx-4 max-h-[90vh] flex flex-col">
        <!-- Header fisso -->
        <div class="p-6 pb-4 border-b bg-white rounded-t-xl flex-shrink-0">
          <div class="flex items-center justify-between">
            <h3 class="text-xl font-semibold text-gray-900 flex items-center">
              <svg class="w-6 h-6 mr-2 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"></path>
              </svg>
              {alarmModalMode === 'add' ? $_('alarms.add_alarm') : $_('alarms.edit_alarm')}
            </h3>
            <button
              on:click={closeAlarmModal}
              class="text-gray-400 hover:text-gray-600 transition-colors">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
        </div>

        <!-- Form Fields scrollabile -->
        <div class="flex-1 overflow-y-auto p-6">
          <div class="grid grid-cols-2 gap-4">
            <!-- Nome Allarme -->
            <div class="col-span-2">
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.alarm_name')}</label>
              <input type="text" bind:value={currentAlarm.alarm_name} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Variabile con selezione -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.alarm_variable')}</label>
              <div class="flex gap-2">
                <input type="text" bind:value={currentAlarm.variable} 
                  class="flex-1 border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
                <button
                  on:click={openVariableModal}
                  class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-2 rounded-lg flex items-center gap-1">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                  </svg>
                  {$_('alarms.select')}
                </button>
              </div>
            </div>

            <!-- Device -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.alarm_device')}</label>
              <input type="text" bind:value={currentAlarm.device} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Area -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.alarm_area')}</label>
              <input type="text" bind:value={currentAlarm.area} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Enabled -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.alarm_enabled')}</label>
              <select bind:value={currentAlarm.enabled} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                <option value="True">{$_('alarms.true')}</option>
                <option value="False">{$_('alarms.false')}</option>
              </select>
            </div>

            <!-- Threshold Name -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.threshold_name')}</label>
              <input type="text" bind:value={currentAlarm.threshold_name} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Threshold Title (chiave traduzione) -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.threshold_title')}</label>
              <div class="flex gap-2">
                <input type="text" bind:value={currentAlarm.threshold_title} 
                  class="flex-1 border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
                <button
                  on:click={() => openTranslationKeyModal('threshold_title')}
                  class="bg-purple-500 hover:bg-purple-600 text-white px-3 py-2 rounded-lg flex items-center gap-1">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                  </svg>
                  {$_('alarms.select')}
                </button>
              </div>
            </div>

            <!-- Threshold Help -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.threshold_help')}</label>
              <div class="flex gap-2">
                <input type="text" bind:value={currentAlarm.threshold_help} 
                  class="flex-1 border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
                <button
                  on:click={() => openTranslationKeyModal('threshold_help')}
                  class="bg-purple-500 hover:bg-purple-600 text-white px-3 py-2 rounded-lg flex items-center gap-1">
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
                  </svg>
                  {$_('alarms.select')}
                </button>
              </div>
            </div>

            <!-- Severity -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.severity')}</label>
              <select bind:value={currentAlarm.severity} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                <option value="0">{$_('alarms.severity_low')}</option>
                <option value="1">{$_('alarms.severity_medium')}</option>
                <option value="2">{$_('alarms.severity_high')}</option>
                <option value="3">{$_('alarms.severity_critical')}</option>
              </select>
            </div>

            <!-- Condition -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.condition')}</label>
              <select bind:value={currentAlarm.condition} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
                <option value="0">&gt;= ({$_('alarms.condition_greater_equal')})</option>
                <option value="1">&gt; ({$_('alarms.condition_greater')})</option>
                <option value="2">== ({$_('alarms.condition_equal')})</option>
                <option value="3">&lt;= ({$_('alarms.condition_less_equal')})</option>
                <option value="4">&lt; ({$_('alarms.condition_less')})</option>
                <option value="5">{$_('alarms.condition_in_range')}</option>
                <option value="6">{$_('alarms.condition_out_range')}</option>
                <option value="7">!= ({$_('alarms.condition_not_equal')})</option>
              </select>
            </div>

            <!-- Threshold Value -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.threshold_value')}</label>
              <input type="text" bind:value={currentAlarm.threshold_value} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Sec Delay -->
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">{$_('alarms.sec_delay')}</label>
              <input type="number" bind:value={currentAlarm.sec_delay} 
                class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:ring-2 focus:ring-blue-500 focus:border-blue-500" />
            </div>

            <!-- Boolean options in a row -->
            <div class="col-span-2 grid grid-cols-5 gap-4 mt-2">
              <div class="flex items-center gap-2">
                <input type="checkbox" id="support_ack" 
                  checked={currentAlarm.support_ack === 'True'}
                  on:change={(e) => currentAlarm.support_ack = e.target.checked ? 'True' : 'False'}
                  class="w-4 h-4 text-blue-600 rounded" />
                <label for="support_ack" class="text-sm text-gray-700">{$_('alarms.support_ack')}</label>
              </div>
              <div class="flex items-center gap-2">
                <input type="checkbox" id="support_reset" 
                  checked={currentAlarm.support_reset === 'True'}
                  on:change={(e) => currentAlarm.support_reset = e.target.checked ? 'True' : 'False'}
                  class="w-4 h-4 text-blue-600 rounded" />
                <label for="support_reset" class="text-sm text-gray-700">{$_('alarms.support_reset')}</label>
              </div>
              <div class="flex items-center gap-2">
                <input type="checkbox" id="log" 
                  checked={currentAlarm.log === 'True'}
                  on:change={(e) => currentAlarm.log = e.target.checked ? 'True' : 'False'}
                  class="w-4 h-4 text-blue-600 rounded" />
                <label for="log" class="text-sm text-gray-700">{$_('alarms.log')}</label>
              </div>
              <div class="flex items-center gap-2">
                <input type="checkbox" id="print" 
                  checked={currentAlarm.print === 'True'}
                  on:change={(e) => currentAlarm.print = e.target.checked ? 'True' : 'False'}
                  class="w-4 h-4 text-blue-600 rounded" />
                <label for="print" class="text-sm text-gray-700">{$_('alarms.print')}</label>
              </div>
              <div class="flex items-center gap-2">
                <input type="checkbox" id="beep_enabled" 
                  checked={currentAlarm.beep_enabled === 'True'}
                  on:change={(e) => currentAlarm.beep_enabled = e.target.checked ? 'True' : 'False'}
                  class="w-4 h-4 text-blue-600 rounded" />
                <label for="beep_enabled" class="text-sm text-gray-700">{$_('alarms.beep_enabled')}</label>
              </div>
            </div>

            <!-- Alarm Type Selection -->
            <div class="col-span-2 mt-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
              <h4 class="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01"></path>
                </svg>
                {$_('alarms.alarm_type')}
              </h4>
              <div class="flex gap-3">
                <!-- Messaggio Rosso button -->
                <button
                  type="button"
                  on:click={() => {
                    currentAlarm.back_color = '4294967295';
                    currentAlarm.text_color = '255';
                    currentAlarm.blink_back_color = '4294967295';
                    currentAlarm.blink_text_color = '4294967295';
                    currentAlarm.print = 'True';
                    currentAlarm.log = 'True';
                    currentAlarm.blink_on_new_alarm = 'True';
                    currentAlarm.support_ack = 'True';
                    currentAlarm.support_reset = 'True';
                    currentAlarm.beep_enabled = 'True';
                  }}
                  class="flex-1 p-3 rounded-lg border-2 transition-all {
                    currentAlarm.text_color === '255'
                      ? 'border-red-500 bg-red-50 ring-2 ring-red-200' 
                      : 'border-gray-300 bg-white hover:border-red-300 hover:bg-red-50'
                  }">
                  <div class="flex items-center gap-2">
                    <div class="p-2 rounded-full bg-red-100">
                      <svg class="w-5 h-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                      </svg>
                    </div>
                    <div class="text-left">
                      <div class="font-semibold text-gray-800 text-sm">{$_('alarms.type_red')}</div>
                      <div class="text-xs text-gray-500">{$_('alarms.type_red_desc')}</div>
                    </div>
                  </div>
                </button>
                
                <!-- Messaggio Giallo button -->
                <button
                  type="button"
                  on:click={() => {
                    currentAlarm.back_color = '0';
                    currentAlarm.text_color = '65535';
                    currentAlarm.blink_back_color = '13922560';
                    currentAlarm.blink_text_color = '0';
                    currentAlarm.print = 'True';
                    currentAlarm.log = 'False';
                    currentAlarm.blink_on_new_alarm = 'True';
                    currentAlarm.support_ack = 'False';
                    currentAlarm.support_reset = 'False';
                    currentAlarm.beep_enabled = 'True';
                  }}
                  class="flex-1 p-3 rounded-lg border-2 transition-all {
                    currentAlarm.text_color === '65535'
                      ? 'border-yellow-500 bg-yellow-50 ring-2 ring-yellow-200' 
                      : 'border-gray-300 bg-white hover:border-yellow-300 hover:bg-yellow-50'
                  }">
                  <div class="flex items-center gap-2">
                    <div class="p-2 rounded-full bg-yellow-100">
                      <svg class="w-5 h-5 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                    </div>
                    <div class="text-left">
                      <div class="font-semibold text-gray-800 text-sm">{$_('alarms.type_yellow')}</div>
                      <div class="text-xs text-gray-500">{$_('alarms.type_yellow_desc')}</div>
                    </div>
                  </div>
                </button>
                
                <!-- Messaggio Verde button -->
                <button
                  type="button"
                  on:click={() => {
                    currentAlarm.back_color = '0';
                    currentAlarm.text_color = '65280';
                    currentAlarm.blink_back_color = '13922560';
                    currentAlarm.blink_text_color = '0';
                    currentAlarm.print = 'True';
                    currentAlarm.log = 'True';
                    currentAlarm.blink_on_new_alarm = 'False';
                    currentAlarm.support_ack = 'False';
                    currentAlarm.support_reset = 'False';
                    currentAlarm.beep_enabled = 'False';
                  }}
                  class="flex-1 p-3 rounded-lg border-2 transition-all {
                    currentAlarm.text_color === '65280'
                      ? 'border-green-500 bg-green-50 ring-2 ring-green-200' 
                      : 'border-gray-300 bg-white hover:border-green-300 hover:bg-green-50'
                  }">
                  <div class="flex items-center gap-2">
                    <div class="p-2 rounded-full bg-green-100">
                      <svg class="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                    </div>
                    <div class="text-left">
                      <div class="font-semibold text-gray-800 text-sm">{$_('alarms.type_green')}</div>
                      <div class="text-xs text-gray-500">{$_('alarms.type_green_desc')}</div>
                    </div>
                  </div>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer fisso -->
        <div class="p-6 pt-4 border-t bg-white rounded-b-xl flex-shrink-0">
          <div class="flex justify-end gap-3">
            <button
              on:click={closeAlarmModal}
              class="bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-6 rounded-lg">
              {$_('alarms.cancel')}
            </button>
            <button
              on:click={saveAlarm}
              class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-6 rounded-lg flex items-center gap-2">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
              </svg>
              {alarmModalMode === 'add' ? $_('alarms.create') : $_('alarms.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal per selezione variabile -->
  {#if showVariableModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-[60] flex items-center justify-center">
      <div class="relative bg-white rounded-xl shadow-2xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col">
        <div class="p-4 border-b">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">{$_('alarms.select_variable')}</h3>
            <button on:click={closeVariableModal} class="text-gray-400 hover:text-gray-600">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <!-- Search -->
          <div class="mt-3 relative">
            <input type="text" bind:value={variableSearchTerm} on:input={filterVariables}
              placeholder="{$_('alarms.search_variables')}"
              class="w-full border border-gray-300 rounded-lg px-10 py-2 focus:ring-2 focus:ring-blue-500" />
            <svg class="w-5 h-5 text-gray-400 absolute left-3 top-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
            </svg>
          </div>
        </div>
        
        <div class="flex-1 overflow-y-auto p-4">
          {#if filteredVariables.length === 0}
            <div class="text-center py-8 text-gray-500">
              <svg class="mx-auto w-12 h-12 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              {$_('alarms.no_variables_found')}
            </div>
          {:else}
            <div class="space-y-2">
              {#each filteredVariables as variable}
                <button
                  on:click={() => selectVariable(variable)}
                  class="w-full text-left p-3 rounded-lg border border-gray-200 hover:border-blue-500 hover:bg-blue-50 transition-colors">
                  <div class="flex items-center justify-between">
                    <div>
                      <span class="font-medium text-gray-900">{variable.name}</span>
                      <span class="ml-2 text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-600">
                        {getVariableTypeLabel(variable.var_type)}
                      </span>
                    </div>
                    {#if variable.var_group}
                      <span class="text-xs text-gray-500">{variable.var_group}</span>
                    {/if}
                  </div>
                  {#if variable.description}
                    <p class="text-sm text-gray-500 mt-1 truncate">{variable.description}</p>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="p-4 border-t">
          <button on:click={closeVariableModal}
            class="w-full bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded-lg">
            {$_('alarms.cancel')}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Modal per selezione chiave traduzione -->
  {#if showTranslationKeyModal}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-[60] flex items-center justify-center">
      <div class="relative bg-white rounded-xl shadow-2xl max-w-3xl w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col">
        <!-- Header fisso -->
        <div class="p-4 border-b flex-shrink-0">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-gray-900">{$_('alarms.select_translation_key')}</h3>
            <button on:click={closeTranslationKeyModal} class="text-gray-400 hover:text-gray-600">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
          <!-- Search -->
          <div class="mt-3 relative">
            <input type="text" bind:value={translationKeySearchTerm} on:input={filterTranslationKeys}
              placeholder="{$_('alarms.search_translation_keys')}"
              class="w-full border border-gray-300 rounded-lg px-10 py-2 focus:ring-2 focus:ring-purple-500" />
            <svg class="w-5 h-5 text-gray-400 absolute left-3 top-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
            </svg>
          </div>
          
          <!-- Language tabs -->
          {#if translationLanguages.length > 0}
            <div class="mt-3 flex flex-wrap gap-1 border-b border-gray-200">
              {#each translationLanguages as lang}
                <button
                  on:click={() => selectedTranslationLanguage = lang}
                  class="px-3 py-2 text-sm font-medium rounded-t-lg transition-colors
                    {selectedTranslationLanguage === lang 
                      ? 'bg-purple-100 text-purple-700 border-b-2 border-purple-500' 
                      : 'text-gray-500 hover:text-gray-700 hover:bg-gray-100'}">
                  {lang.toUpperCase()}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        
        <!-- Lista scrollabile -->
        <div class="flex-1 overflow-y-auto p-4">
          {#if filteredTranslationKeys.length === 0}
            <div class="text-center py-8 text-gray-500">
              <svg class="mx-auto w-12 h-12 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              {$_('alarms.no_translation_keys_found')}
            </div>
          {:else}
            <div class="space-y-2">
              {#each filteredTranslationKeys as keyObj}
                <button
                  on:click={() => selectTranslationKey(keyObj)}
                  class="w-full text-left p-3 rounded-lg border border-gray-200 hover:border-purple-500 hover:bg-purple-50 transition-colors">
                  <div class="flex items-center justify-between">
                    <span class="font-medium text-gray-900">{keyObj.key}</span>
                    <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                    </svg>
                  </div>
                  {#if selectedTranslationLanguage && keyObj[selectedTranslationLanguage]}
                    <p class="text-sm text-gray-600 mt-1 line-clamp-2">
                      <span class="text-purple-600 font-medium">{selectedTranslationLanguage.toUpperCase()}:</span> {keyObj[selectedTranslationLanguage]}
                    </p>
                  {:else if selectedTranslationLanguage}
                    <p class="text-sm text-gray-400 mt-1 italic">
                      {$_('alarms.no_translation_available')}
                    </p>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Footer fisso -->
        <div class="p-4 border-t flex-shrink-0">
          <button on:click={closeTranslationKeyModal}
            class="w-full bg-gray-500 hover:bg-gray-600 text-white font-bold py-2 px-4 rounded-lg">
            {$_('alarms.cancel')}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>