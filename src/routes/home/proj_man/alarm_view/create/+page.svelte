<script>
  import { invoke } from '@tauri-apps/api/core';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';

  let table = '';
  let fields = [];
  let nuovo = {};
  let dirty = false;
  let showToast = false;
  let toastMsg = '';
  let toastType = 'success';

  onMount(async () => {
    const urlParams = $page.url.searchParams;
    table = urlParams.get('table') || '';
    if (table) {
      try {
        fields = await invoke('get_table_columns', { tableName: table });
        fields = fields.filter(f => f !== 'id' && f !== 'image'); // Exclude id and image
        // Sort fields: desc, code, enable first, then others
        const priority = ['desc', 'code', 'enable'];
        fields.sort((a, b) => {
          const aPri = priority.indexOf(a);
          const bPri = priority.indexOf(b);
          if (aPri !== -1 && bPri !== -1) return aPri - bPri;
          if (aPri !== -1) return -1;
          if (bPri !== -1) return 1;
          return a.localeCompare(b);
        });
        // Initialize nuovo
        nuovo = {};
        for (let f of fields) {
          nuovo[f] = (f === 'enable' || f === 'abilitato' || f === 'attivo') ? false : '';
        }
      } catch (e) {
        console.error('Errore caricamento colonne:', e);
        toastMsg = $_('table.error_loading_columns');
        toastType = 'error';
        showToast = true;
        setTimeout(() => { showToast = false; }, 2000);
      }
    }
  });

  function handleChange() {
    dirty = Object.values(nuovo).some(v => v !== '' && v !== false);
  }

  async function save() {
    try {
      // Convert record to strings
      let recordToSave = {};
      for (let f of fields) {
        if (f === 'enable' || f === 'abilitato' || f === 'attivo') {
          recordToSave[f] = nuovo[f] ? 'true' : 'false';
        } else {
          recordToSave[f] = nuovo[f];
        }
      }
      await invoke('insert_record', { tableName: table, record: recordToSave });
      toastMsg = $_('table.record_created');
      toastType = 'success';
      // Reset campi
      nuovo = {};
      for (let f of fields) {
        nuovo[f] = (f === 'enable' || f === 'abilitato' || f === 'attivo') ? false : '';
      }
      dirty = false;
      // Redirect to table page
      window.location.href = `/home/proj_view?table=${encodeURIComponent(table)}`;
    } catch (e) {
      console.error('Errore nella creazione:', e);
      toastMsg = $_('table.error_creating') + e;
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
        <h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('table.create')}</h1>
        <p class="text-gray-700 text-sm">{$_('table.create_description')}</p>
      </div>
      <div class="flex items-center gap-2 justify-end">
        <button class="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-6 rounded disabled:bg-gray-300 disabled:text-gray-500 flex items-center gap-2" on:click={save} disabled={!dirty}>
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"></path>
          </svg>
          {$_('table.save')}
        </button>
      </div>
    </div>
  </header>
  <main class="flex-grow flex flex-col justify-start items-center gap-4 px-5 mb-8 w-full" style="padding-top: 120px; margin-bottom: 2rem;">
    <div class="w-full max-w-7xl px-4 sm:px-6 lg:px-8 bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg mx-auto">
      <ul>
        {#each fields as col, i}
          <li class="flex items-center gap-4 py-4 my-4 bg-white/95 rounded-lg border border-gray-200 shadow-sm">
            <span class="w-1/3 text-base font-semibold text-gray-900 text-left px-4">{col}</span>
            <span class="flex-1 flex justify-end px-4">
              {#if col === 'enable' || col === 'abilitato' || col === 'attivo'}
                <input type="checkbox" bind:checked={nuovo[col]} on:change={handleChange} class="h-5 w-5" />
              {:else}
                <input type="text" bind:value={nuovo[col]} on:input={handleChange} class="border rounded px-3 py-2 w-full max-w-xs text-left" />
              {/if}
            </span>
          </li>
          {#if i === 2}
            <hr class="my-2 border-gray-300">
          {/if}
        {/each}
      </ul>
    </div>
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
