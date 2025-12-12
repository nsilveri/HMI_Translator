<script>
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save } from '@tauri-apps/plugin-dialog';
	import { open as openUrl } from '@tauri-apps/plugin-shell';
	import { _ } from 'svelte-i18n';
	import { locale } from 'svelte-i18n';

	let theme = "light";
	let notifications = true;
	let language = "en";
	let showToast = false;
	let toastMsg = '';
	let toastType = 'success';
	
	// Impostazioni per i servizi di traduzione
	let translationService = "deepl";
	let deeplApiKey = "";
	let googleApiKey = "";
	let microsoftApiKey = "";
	let microsoftRegion = "westeurope";
	
	// Impostazione percorso macchina
	let machinePath = "";

	onMount(async () => {
		try {
			language = await invoke('get_setting', { key: 'language' }) || 'en';
			$locale = language;
			
			// Carica le impostazioni per i servizi di traduzione
			translationService = await invoke('get_setting', { key: 'translation_service' }) || 'deepl';
			deeplApiKey = await invoke('get_setting', { key: 'deepl_api_key' }) || '';
			googleApiKey = await invoke('get_setting', { key: 'google_api_key' }) || '';
			microsoftApiKey = await invoke('get_setting', { key: 'microsoft_api_key' }) || '';
			microsoftRegion = await invoke('get_setting', { key: 'microsoft_region' }) || 'westeurope';
			
			// Carica il percorso macchina
			machinePath = await invoke('get_setting', { key: 'machine_path' }) || '';
		} catch (e) {
			console.error($_('settings.error_loading_api'), e);
		}
	});

	async function saveSettings() {
		try {
			await invoke('set_setting', { key: 'language', value: language });
			$locale = language;
			
			// Salva le impostazioni per i servizi di traduzione
			await invoke('set_setting', { key: 'translation_service', value: translationService });
			await invoke('set_setting', { key: 'deepl_api_key', value: deeplApiKey });
			await invoke('set_setting', { key: 'google_api_key', value: googleApiKey });
			await invoke('set_setting', { key: 'microsoft_api_key', value: microsoftApiKey });
			await invoke('set_setting', { key: 'microsoft_region', value: microsoftRegion });
			
			// Salva il percorso macchina
			await invoke('set_setting', { key: 'machine_path', value: machinePath });
			
			toastMsg = $_('settings.settings_saved');
			toastType = 'success';
			showToast = true;
			setTimeout(() => { showToast = false; }, 2500);
		} catch (e) {
			console.error('Errore nel salvataggio:', e);
			toastMsg = $_('settings.error_saving') + e;
			toastType = 'error';
			showToast = true;
			setTimeout(() => { showToast = false; }, 2500);
		}
	}

	async function browseMachinePath() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: $_('settings.machine_path')
			});
			if (selected) {
				machinePath = selected;
			}
		} catch (e) {
			console.error('Error browsing for machine path:', e);
		}
	}

	async function exportDatabase() {
		try {
			const filePath = await save({
				defaultPath: 'projects_backup.db',
				filters: [{
					name: 'Database files',
					extensions: ['db']
				}],
				title: $_('settings.export_database_title')
			});
			
			if (filePath) {
				const result = await invoke('export_database', { filePath });
				toastMsg = $_('settings.export_database_success');
				toastType = 'success';
				showToast = true;
				setTimeout(() => { showToast = false; }, 3000);
			}
		} catch (e) {
			console.error('Error exporting database:', e);
			toastMsg = $_('settings.export_database_error') + ' ' + e;
			toastType = 'error';
			showToast = true;
			setTimeout(() => { showToast = false; }, 5000);
		}
	}

	async function importDatabase() {
		try {
			const filePath = await open({
				filters: [{
					name: 'Database files',
					extensions: ['db']
				}],
				title: $_('settings.import_database_title'),
				multiple: false
			});
			
			if (filePath) {
				// Mostra un messaggio di conferma
				if (confirm($_('settings.import_database_confirm'))) {
					const result = await invoke('import_database', { filePath });
					toastMsg = $_('settings.import_database_success');
					toastType = 'success';
					showToast = true;
					setTimeout(() => { showToast = false; }, 3000);
					
					// Ricarica la pagina per riflettere i cambiamenti
					setTimeout(() => {
						window.location.reload();
					}, 1000);
				}
			}
		} catch (e) {
			console.error('Error importing database:', e);
			toastMsg = $_('settings.import_database_error') + ' ' + e;
			toastType = 'error';
			showToast = true;
			setTimeout(() => { showToast = false; }, 5000);
		}
	}

	async function openTranslationAPIWebsite() {
		let url = '';
		switch (translationService) {
			case 'deepl':
				url = 'https://www.deepl.com/pro-api';
				break;
			case 'google':
				url = 'https://cloud.google.com/translate/docs/setup';
				break;
			case 'microsoft':
				url = 'https://azure.microsoft.com/en-us/services/cognitive-services/translator/';
				break;
			default:
				url = 'https://www.deepl.com/pro-api';
		}
		
		try {
			await openUrl(url);
		} catch (e) {
			console.error('Error opening URL:', e);
		}
	}
</script>

<div class="relative min-h-screen flex flex-col" style="min-height: 100vh;">
	<div class="absolute inset-0 w-full h-full" style="background: linear-gradient(135deg, #f0f0f0 0%, #c9ffe7 70%, #e0c9ff 100%); opacity: 0.7; backdrop-filter: blur(12px); z-index: 0; pointer-events: none;"></div>
	
	<!-- Header fisso -->
	<header class="w-full pt-5 px-5 fixed top-0 left-0 right-0 z-10 bg-transparent">
		<div class="w-full bg-white/50 backdrop-blur-sm rounded-lg border border-black/50 p-4 sm:p-2 text-center shadow-lg mx-auto">
			<div class="mb-1">
				<h1 class="text-2xl font-semibold text-gray-900 mb-1">{$_('settings.title')}</h1>
				<p class="text-gray-700">
					{$_('settings.description')}
				</p>
			</div>
		</div>
	</header>
	
	<!-- Main scrollabile con margin-top per compensare l'header fisso -->
	<main class="flex-grow flex justify-center items-start px-4 py-6 overflow-y-auto" style="margin-top: 7rem; margin-bottom: 1rem;">
		<div class="max-w-4xl w-full space-y-6">
			
			<!-- Sezione Generale -->
			<div class="bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg">
				<h2 class="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
					</svg>
					{$_('settings.general_settings')}
				</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
					<div>
						<label class="block font-medium mb-1">{$_('settings.language')}</label>
						<select bind:value={language} class="border rounded px-3 py-2 w-full">
							<option value="en">{$_('settings.english')}</option>
							<option value="it">{$_('settings.italian')}</option>
						</select>
					</div>
				</div>

			</div>

			<!-- Sezione Servizi di Traduzione -->
			<div class="bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg">
				<h2 class="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"></path>
					</svg>
					{$_('settings.translation_settings')}
				</h2>
				
				<div class="mb-4">
					<label class="block font-medium mb-2">{$_('settings.translation_service')}</label>
					<select bind:value={translationService} class="border rounded px-3 py-2 w-full max-w-xs">
						<option value="deepl">{$_('settings.deepl')}</option>
						<option value="google">{$_('settings.google_translate')}</option>
						<option value="microsoft">{$_('settings.microsoft_translator')}</option>
					</select>
				</div>

				{#if translationService === 'deepl'}
					<div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
						<h3 class="font-medium text-blue-800 mb-2 flex items-center gap-2">
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
							</svg>
							{$_('settings.deepl_config')}
						</h3>
						<div class="space-y-3">
							<div>
								<label class="block font-medium mb-1">{$_('settings.deepl_api_key_label')}</label>
								<input 
									type="text" 
									bind:value={deeplApiKey} 
									placeholder="{$_('settings.deepl_placeholder')}" 
									class="border rounded px-3 py-2 w-full" />
								<p class="text-sm text-gray-600 mt-1">
									{$_('settings.deepl_description')}
									<button type="button" class="text-blue-500 underline hover:text-blue-700" on:click={openTranslationAPIWebsite}>{$_('settings.deepl_link_text')}</button>
								</p>
								{#if deeplApiKey}
									<p class="text-xs text-green-600 mt-1">{$_('settings.api_key_status', { values: { count: deeplApiKey.length } })}</p>
								{/if}
							</div>
						</div>
					</div>
				{:else if translationService === 'google'}
					<div class="bg-green-50 border border-green-200 rounded-lg p-4">
						<h3 class="font-medium text-green-800 mb-2 flex items-center gap-2">
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
							</svg>
							{$_('settings.google_config')}
						</h3>
						<div class="space-y-3">
							<div>
								<label class="block font-medium mb-1">{$_('settings.google_api_key_label')}</label>
								<input 
									type="text" 
									bind:value={googleApiKey} 
									placeholder="{$_('settings.google_placeholder')}" 
									class="border rounded px-3 py-2 w-full" />
								<p class="text-sm text-gray-600 mt-1">
									{$_('settings.google_description')}
									<button type="button" class="text-blue-500 underline hover:text-blue-700" on:click={openTranslationAPIWebsite}>{$_('settings.google_link_text')}</button>
								</p>
								{#if googleApiKey}
									<p class="text-xs text-green-600 mt-1">{$_('settings.api_key_status', { values: { count: googleApiKey.length } })}</p>
								{/if}
							</div>
						</div>
					</div>
				{:else if translationService === 'microsoft'}
					<div class="bg-purple-50 border border-purple-200 rounded-lg p-4">
						<h3 class="font-medium text-purple-800 mb-2 flex items-center gap-2">
							<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
							</svg>
							{$_('settings.microsoft_config')}
						</h3>
						<div class="space-y-3">
							<div>
								<label class="block font-medium mb-1">{$_('settings.microsoft_api_key_label')}</label>
								<input 
									type="text" 
									bind:value={microsoftApiKey} 
									placeholder="{$_('settings.microsoft_placeholder')}" 
									class="border rounded px-3 py-2 w-full" />
								{#if microsoftApiKey}
									<p class="text-xs text-green-600 mt-1">{$_('settings.api_key_status', { values: { count: microsoftApiKey.length } })}</p>
								{/if}
							</div>
							<div>
								<label class="block font-medium mb-1">{$_('settings.azure_region')}</label>
								<select bind:value={microsoftRegion} class="border rounded px-3 py-2 w-full max-w-xs">
									<option value="westeurope">West Europe</option>
									<option value="eastus">East US</option>
									<option value="westus2">West US 2</option>
									<option value="southeastasia">Southeast Asia</option>
								</select>
							</div>
							<p class="text-sm text-gray-600">
								{$_('settings.microsoft_description')}
								<button type="button" class="text-blue-500 underline hover:text-blue-700" on:click={openTranslationAPIWebsite}>{$_('settings.microsoft_link_text')}</button>
							</p>
						</div>
					</div>
				{/if}
			</div>

			<!-- Sezione Impostazioni Macchina -->
			<div class="bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg">
				<h2 class="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
					</svg>
					{$_('settings.machine_settings')}
				</h2>
				
				<div class="bg-orange-50 border border-orange-200 rounded-lg p-4">
					<div class="space-y-3">
						<div>
							<label class="block font-medium mb-1">{$_('settings.machine_path')}</label>
							<div class="flex gap-2">
								<input 
									type="text" 
									bind:value={machinePath} 
									placeholder="{$_('settings.machine_path_placeholder')}" 
									class="border rounded px-3 py-2 flex-1" />
								<button 
									type="button"
									on:click={browseMachinePath}
									class="bg-orange-500 hover:bg-orange-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
									<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
									</svg>
									{$_('settings.browse')}
								</button>
							</div>
							<p class="text-sm text-gray-600 mt-1">
								{$_('settings.machine_path_description')}
							</p>
							{#if machinePath}
								<p class="text-xs text-green-600 mt-1">{$_('settings.machine_path_set')}</p>
							{/if}
						</div>
					</div>
				</div>
			</div>

			<!-- Sezione Database -->
			<div class="bg-white/90 backdrop-blur-sm rounded-lg border border-white/20 p-6 shadow-lg">
				<h2 class="text-xl font-semibold text-gray-800 mb-4 flex items-center gap-2">
					<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
					</svg>
					{$_('settings.database_settings')}
				</h2>
				
				<div class="bg-red-50 border border-red-200 rounded-lg p-4">
					<h3 class="font-medium text-red-800 mb-3 flex items-center gap-2">
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"></path>
						</svg>
						{$_('settings.database_backup_warning')}
					</h3>
					<div class="space-y-3">
						<div>
							<label class="block font-medium mb-1">{$_('settings.export_database')}</label>
							<p class="text-sm text-gray-600 mb-2">
								{$_('settings.export_database_description')}
							</p>
							<button 
								type="button"
								on:click={exportDatabase}
								class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
								<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
								</svg>
								{$_('settings.export_database_button')}
							</button>
						</div>
						<div>
							<label class="block font-medium mb-1">{$_('settings.import_database')}</label>
							<p class="text-sm text-gray-600 mb-2">
								{$_('settings.import_database_description')}
							</p>
							<button 
								type="button"
								on:click={importDatabase}
								class="bg-red-500 hover:bg-red-600 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
								<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M12 2v6m0 0l-3-3m3 3l3-3"></path>
								</svg>
								{$_('settings.import_database_button')}
							</button>
						</div>
					</div>
				</div>
			</div>
		</div>
	</main>
</div>



{#if showToast}
	<div class="fixed bottom-4 right-4 z-50 text-white px-4 py-2 rounded shadow-lg" class:bg-green-500={toastType === 'success'} class:bg-red-500={toastType === 'error'}>
		{toastMsg}
	</div>
{/if}
