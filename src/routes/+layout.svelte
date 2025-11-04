<script>
	import '../app.css';
	import '../lib/i18n';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { locale } from 'svelte-i18n';
	import { _ } from 'svelte-i18n';

	let localeReady = false;

	onMount(async () => {
		try {
			const lang = await invoke('get_setting', { key: 'language' }) || 'en';
			$locale = lang;
			// Also save to localStorage for faster initial load
			if (typeof window !== 'undefined') {
				localStorage.setItem('language', lang);
			}
			localeReady = true;
		} catch (e) {
			console.error('Errore nel caricamento della lingua:', e);
			$locale = 'en';
			localeReady = true;
		}
	});
</script>
<!--
<header>
	<nav class="fixed top-0 left-0 right-0 bg-blue-200 shadow">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between h-8">
				<div class="flex">
					<a href="/" class="flex-shrink-0 flex items-center text-xl font-bold text-center text-gray-900">
						HMI Translator
					</a>
				</div>
			</div>
		</div>

	</nav>
</header>
-->

{#if localeReady}
<main class="h-full overflow-y-auto">
	<slot />
</main>

<footer class="fixed bottom-0 left-0 right-0 z-10 bg-transparent px-5 pb-2">
	<div class="w-full bg-white/50 backdrop-blur-sm rounded-lg border border-black/50 p-4 sm:p-2 text-center shadow-lg mx-auto flex justify-center gap-4">
		<a href="/home" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path>
			</svg>
			{$_('footer.home')}
		</a>
		<a href="/settings" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
			</svg>
			{$_('footer.settings')}
		</a>
		<!-- About page temporaneamente nascosta
		<a href="/about" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded flex items-center gap-2">
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
			</svg>
			{$_('footer.about')}
		</a>
		-->
	</div>
</footer>
{:else}
<div class="h-screen flex items-center justify-center">
	<div class="text-center">
		<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto mb-4"></div>
		<p class="text-gray-600">{$_('home.loading')}</p>
	</div>
</div>
{/if}