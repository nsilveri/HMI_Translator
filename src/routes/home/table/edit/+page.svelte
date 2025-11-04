<script>
  // @ts-nocheck
  import { invoke } from '@tauri-apps/api/core';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';

  let record = {};
  let edited = {};
  let dirty = false;
  let loading = true;
  let error = '';

  $: tableName = $page.url.searchParams.get('table');
  $: id = decodeURIComponent($page.url.searchParams.get('id') || '');

  onMount(async () => {
    if (!tableName) {
      error = 'Nome tabella mancante nell\'URL';
      loading = false;
      return;
    }
    if (!id) {
      error = 'ID mancante nell\'URL';
      loading = false;
      return;
    }
    try {
      const records = await invoke('get_records', { tableName });
      const found = records.find(r => String(r.id) === id || r.id === id);
      if (found) {
        record = { ...found };
        edited = { ...found };
        delete edited.id; // Don't edit id
      } else {
        error = `Record con id "${id}" non trovato nella tabella "${tableName}". Records totali: ${records.length}`;
      }
    } catch (e) {
      error = $_('table.error_loading_record') + e;
    }
    loading = false;
  });

  function handleChange() {
    dirty = JSON.stringify(edited) !== JSON.stringify(record);
  }

  let showToast = false;
  let toastMsg = '';
  let toastType = 'success'; // 'success' | 'error'

  async function salva() {
    try {
      // Calculate changes
      const changes = {};
      for (const [key, val] of Object.entries(edited)) {
        if (record[key] !== val) {
          changes[key] = val;
        }
      }
      if (Object.keys(changes).length > 0) {
              await invoke('update_record', { tableName, id: String(record.id), updates: changes });
        Object.assign(record, edited);
        dirty = false;
        toastMsg = $_('table.record_updated');
        toastType = 'success';
        // Go back after success
        setTimeout(() => { window.history.back(); }, 500);
      } else {
        toastMsg = $_('database.no_changes');
        toastType = 'success';
      }
    } catch (e) {
      toastMsg = $_('table.error_updating') + e;
      toastType = 'error';
    }
    showToast = true;
    setTimeout(() => { showToast = false; }, 2000);
  }
</script>

<div class="min-h-screen flex flex-col" style="background: linear-gradient(135deg, #c9ffe7 0%, #e9e9ff 70%, #dcecff 100%);">
  <header class="w-full pt-5 px-5 fixed top-0 left-0 right-0 z-10 bg-transparent">
    <div class="w-full max-w-7xl mx-auto bg-white/50 backdrop-blur-sm rounded-lg border border-black/50 p-4 sm:p-2 shadow-lg flex items-center justify-between">
      <div class="flex items-center gap-2">
        <button class="bg-gray-200 hover:bg-gray-300 text-gray-800 font-bold py-2 px-4 rounded flex items-center" on:click={() => window.history.back()}>
          ← {$_('home.back')}
        </button>
      </div>
      <div class="flex-1 text-center">
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('table.edit')}</h1>
        <p class="text-gray-700 text-sm">{$_('table.edit_description')}</p>
      </div>
      <div class="flex items-center gap-2">
        <button class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-6 rounded disabled:bg-gray-300 disabled:text-gray-500 flex items-center gap-2" on:click={salva} disabled={!dirty}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"></path>
          </svg>
          {$_('table.save')}
        </button>
      </div>
    </div>
  </header>

  <main class="flex-grow flex flex-col justify-start items-center gap-4 px-5 mb-8 w-full" style="padding-top: 120px; margin-bottom: 2rem; background: rgba(0,0,0,0.1);">
    {#if loading}
      <div class="w-full max-w-7xl px-4 sm:px-6 lg:px-8 bg-white/50 backdrop-blur-sm rounded-lg border border-white/60 p-6 shadow-lg mx-auto">
        <p class="text-center">{$_('home.loading')}</p>
      </div>
    {:else if error}
      <div class="w-full max-w-7xl px-4 sm:px-6 lg:px-8 bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg mx-auto">
        <p class="text-center text-red-500">{error}</p>
      </div>
    {:else}
      <div class="w-full max-w-7xl px-4 sm:px-6 lg:px-8 bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg mx-auto">
        <ul class="">
          {#each Object.entries(edited).filter(([k]) => k !== 'image').sort(([a], [b]) => {
            if (a === 'desc') return -1;
            if (b === 'desc') return 1;
            if (a === 'code') return -1;
            if (b === 'code') return 1;
            if (a === 'enable') return -1;
            if (b === 'enable') return 1;
            return 0;
          }) as [col, val], i}
            <li class="flex items-center gap-4 py-4 my-4 bg-white/95 rounded-lg border border-gray-200 shadow-sm">
              <span class="w-1/3 text-base font-semibold text-gray-900 text-left px-4">{col}</span>
              <span class="flex-1 flex justify-end px-4">
                {#if col === 'enable' || col === 'abilitato' || col === 'attivo' || val === 'true' || val === 'false'}
                  <input type="checkbox" checked={val === '1' || val === 'true'} on:change={(e) => { edited[col] = e.target.checked ? 'true' : 'false'; handleChange(); }} class="h-5 w-5" />
                {:else}
                  <input type="text" bind:value={edited[col]} on:input={handleChange} class="border rounded px-3 py-2 w-full max-w-xs text-left" />
                {/if}
              </span>
            </li>
            {#if i === 2}
              <hr class="my-2 border-gray-300">
            {/if}
          {/each}
        </ul>
        <!-- Il pulsante Salva ora è nell'header -->
      </div>
    {/if}
  </main>

  {#if showToast}
    <div class="fixed bottom-8 right-8 z-50 px-6 py-3 rounded shadow-lg animate-fadein font-semibold text-white"
      style="background-color: {toastType === 'success' ? '#22c55e' : '#ef4444'};">
      {toastMsg}
    </div>
  {/if}
</div>
<style>
@keyframes fadein {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fadein {
  animation: fadein 0.3s;
}
</style>
