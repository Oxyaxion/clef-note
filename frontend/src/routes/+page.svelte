<script lang="ts">
	import { onMount } from 'svelte';
	import Sidebar from '$lib/Sidebar.svelte';
	import NoteEditorPane from '$lib/NoteEditorPane.svelte';
	import CommandPalette from '$lib/CommandPalette.svelte';
	import Settings from '$lib/Settings.svelte';
	import ShareModal from '$lib/ShareModal.svelte';
	import MetaPage from '$lib/MetaPage.svelte';
	import LoginPage from '$lib/LoginPage.svelte';
	import ConfirmDialog from '$lib/ConfirmDialog.svelte';
	import MobileTopBar from '$lib/MobileTopBar.svelte';
	import {
		listNotes,
		getNote,
		saveNote,
		deleteNote,
		getSettings,
		serializeFrontmatter,
		session,
		logout,
		exchangeOidcCode,
		type NoteMeta,
		type Frontmatter,
	} from '$lib/api';
	import { loadTheme, applyTheme, type ThemeId } from '$lib/theme';
	import { applySettings, DEFAULT, type AppSettings } from '$lib/settings';
	import { setDateFormat } from '$lib/slashCommands';
	import { emit, on } from '$lib/events';
	import { isAbortError } from '$lib/utils';
	import { createNavigation } from '$lib/navigation.svelte';
	import { createAutoSave } from '$lib/autoSave.svelte';

	let notes = $state<NoteMeta[]>([]);
	let selected = $state<string | null>(null);
	let noteContent = $state('');
	let noteFrontmatter = $state<Frontmatter>({});
	let loadCtrl: AbortController | null = null;
	const autoSave = createAutoSave((name, fm) => {
		const pinned = fm.pinned === true;
		const is_index = fm.type === 'index';
		const is_template = fm.type === 'template';
		const cur = notes.find(n => n.name === name);
		if (cur?.pinned === pinned && cur?.is_index === is_index && cur?.is_template === is_template) return;
		notes = notes.map(n =>
			n.name === name ? { ...n, pinned, is_index, is_template } : n
		);
	});
	let paletteOpen = $state(false);
	let sidebarOpen = $state(false);
	let renaming = $state(false);   // bound to NoteEditorPane
	let focusMode = $state(false);  // bound to NoteEditorPane
	let rawView = $state(false);    // raw markdown source view
	let rawDirty = $state(false);   // a raw edit happened → resync on exit
	let reloadNonce = $state(0);    // bumped to force the editor to re-sync content
	let focusEditorNonce = $state(0); // bumped to auto-focus editor (e.g. after note creation)
	let isMobile = $state(false);
	let creatingFromPalette = $state(false);
	let currentTheme = $state<ThemeId>('default');
	let vaultName = $state('Notes');
	let loggedIn = $state(session.exists());
	let oidcError = $state<string | null>(null);
	let settingsOpen = $state(false);
	let metaPageOpen = $state(false);
	let shareModalOpen = $state(false);
	let currentSettings = $state<AppSettings>({ ...DEFAULT });
	const nav = createNavigation();
	let confirmDialog = $state<{ message: string; onConfirm: () => void } | null>(null);
	let loadError = $state<string | null>(null);

	const frontmatterMd = $derived(serializeFrontmatter(noteFrontmatter));
	const noteMarkdown = $derived(frontmatterMd + noteContent);
	// Key for the editor/frontmatter view. Changes on note switch (selected) and
	// after a raw edit (reloadNonce) so the rich editor re-syncs from disk.
	const editorReloadKey = $derived(`${selected}#${reloadNonce}`);

	// Mobile read-only: temporary per-note unlock (not persisted to frontmatter).
	let mobileUnlocked = $state(false);
	$effect(() => { selected; mobileUnlocked = false; });
	const effectiveLocked = $derived(
		noteFrontmatter.locked === true ||
		(isMobile && currentSettings.mobileReadOnly && !mobileUnlocked)
	);
	function toggleLock() {
		if (!selected) return;
		if (noteFrontmatter.locked) {
			const fm = { ...noteFrontmatter };
			delete fm.locked;
			onFrontmatterChange(fm);
		} else if (isMobile && currentSettings.mobileReadOnly) {
			mobileUnlocked = !mobileUnlocked;
		} else {
			onFrontmatterChange({ ...noteFrontmatter, locked: true });
		}
	}

	// Stable reference: only changes when note names actually change, not on every metadata save.
	let _prevNoteNames: string[] = [];
	const noteNames = $derived.by(() => {
		const names = notes.map(n => n.name);
		if (names.length === _prevNoteNames.length && names.every((n, i) => n === _prevNoteNames[i])) {
			return _prevNoteNames;
		}
		_prevNoteNames = names;
		return names;
	});

	// One-time setup: auth expiry + media query — neither depends on reactive state.
	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const oidcCode = params.get('oidc_code');
		const oidcErr = params.get('oidc_error');
		if (oidcCode) {
			history.replaceState({}, '', '/');
			exchangeOidcCode(oidcCode).then(() => { loggedIn = true; }).catch(() => {});
		} else if (oidcErr) {
			history.replaceState({}, '', '/');
			oidcError = oidcErr;
		}

		const offAuth = on(window, 'auth:expired', () => { loggedIn = false; });
		const mq = window.matchMedia('(max-width: 640px)');
		isMobile = mq.matches;
		const mqHandler = (e: MediaQueryListEvent) => { isMobile = e.matches; };
		mq.addEventListener('change', mqHandler);
		return () => { offAuth(); mq.removeEventListener('change', mqHandler); };
	});

	// Re-run when loggedIn changes: apply theme saved in localStorage.
	$effect(() => {
		if (!loggedIn) return;
		const theme = loadTheme();
		currentTheme = theme;
		applyTheme(theme);
	});

	// Re-run when loggedIn changes: fetch initial notes + settings, then open home page.
	$effect(() => {
		if (!loggedIn) return;
		Promise.all([listNotes(), getSettings()]).then(([n, raw]) => {
			notes = n;
			const s: AppSettings = { ...DEFAULT, ...(raw as Partial<AppSettings>) };
			currentSettings = s;
			applySettings(s);
			setDateFormat(s.dateFormat ?? 'long-en');
			vaultName = s.vaultName ?? 'Notes';
			const home = s.homePage?.trim();
			if (home) selectNote(home).catch(() => {});
		});
	});

	// Re-run when loggedIn changes: subscribe to wiki-link navigation events.
	$effect(() => {
		if (!loggedIn) return;
		return on(document, 'wiki-navigate', (target) => {
			if (!target) return;
			const exact = notes.find(n => n.name === target);
			const resolved = exact
				?? notes.find(n => n.name.split('/').pop() === target)
				?? notes.find(n => n.name.toLowerCase().split('/').pop() === target.toLowerCase());
			selectNote(resolved?.name ?? target).catch(() => {});
		});
	});

	async function selectNote(name: string, pushHistory = true) {
		autoSave.flush();
		renaming = false;
		// Keep the raw-source view across note switches (user preference); the new
		// note's pending raw edits start clean and RawSource remounts via reloadKey.
		rawDirty = false;
		loadCtrl?.abort();
		loadCtrl = new AbortController();
		loadError = null;
		try {
			const note = await getNote(name, loadCtrl.signal);
			noteFrontmatter = (note.frontmatter ?? {}) as Frontmatter;
			noteContent = note.content;
			selected = name;
			sidebarOpen = false;
			metaPageOpen = false;
			if (pushHistory) nav.push(name);
		} catch (e) {
			if (isAbortError(e)) return;
			loadError = `Could not load "${name}". The note may have been deleted or the server is unreachable.`;
		}
	}

	function goBack() {
		const target = nav.peekBack();
		if (!target) return;
		selectNote(target, false).then(() => nav.stepBack());
	}

	function goForward() {
		const target = nav.peekForward();
		if (!target) return;
		selectNote(target, false).then(() => nav.stepForward());
	}

	function goHome() {
		const home = currentSettings.homePage?.trim();
		if (!home) return;
		selectNote(home);
	}

	async function createNote(name: string) {
		try {
			await saveNote(name, '');
		} catch {
			loadError = `Could not create "${name}". The server may be unreachable.`;
			return;
		}
		notes = [...notes, { name, pinned: false }];
		await selectNote(name);
		focusEditorNonce++;
	}

	function onEdit(markdown: string) {
		if (!selected) return;
		noteContent = markdown;
		// Auto-sync leading H1 → frontmatter title
		const h1Match = markdown.match(/^#\s+(.+?)(?:\n|$)/);
		if (h1Match) {
			const h1Text = h1Match[1].trim();
			if (noteFrontmatter.title !== h1Text) {
				noteFrontmatter = { ...noteFrontmatter, title: h1Text };
			}
		}
		autoSave.schedule(selected, noteFrontmatter, markdown);
	}

	function onRawEdit(raw: string) {
		if (!selected) return;
		rawDirty = true;
		autoSave.scheduleRaw(selected, raw);
	}

	async function toggleRawView() {
		if (!selected) return;
		if (!rawView) {
			rawView = true;
			return;
		}
		// Leaving the source view: persist the pending raw edit, then re-sync the
		// split frontmatter/body from disk so the rich editor reflects the changes.
		rawView = false;
		await autoSave.flush();
		if (!rawDirty) return;
		rawDirty = false;
		const name = selected;
		try {
			const note = await getNote(name);
			noteFrontmatter = (note.frontmatter ?? {}) as Frontmatter;
			noteContent = note.content;
			reloadNonce++;
			// Refresh sidebar metadata (pinned / index / template may have changed).
			const pinned = noteFrontmatter.pinned === true;
			const is_index = noteFrontmatter.type === 'index';
			const is_template = noteFrontmatter.type === 'template';
			notes = notes.map(n => n.name === name ? { ...n, pinned, is_index, is_template } : n);
		} catch {
			loadError = `Could not reload "${name}" after editing the source.`;
		}
	}

	function onFrontmatterChange(fm: Frontmatter) {
		if (!selected) return;
		noteFrontmatter = fm;
		autoSave.schedule(selected, fm, noteContent);
	}

	function handleRenamed(oldName: string, newName: string) {
		autoSave.cancel();
		notes = notes.map(n => n.name === oldName ? { ...n, name: newName } : n);
		selected = newName;
	}

	function handleDelete() {
		if (!selected) return;
		const name = selected;
		confirmDialog = {
			message: `Delete "${name}"? This action cannot be undone.`,
			onConfirm: async () => {
				confirmDialog = null;
				autoSave.cancel();   // drop any pending edit so we don't resurrect the note
				selected = null;
				noteContent = '';
				noteFrontmatter = {};
				await deleteNote(name);
				notes = notes.filter(n => n.name !== name);
				emit(document, 'notes:changed');
			},
		};
	}

	function openPalette() {
		paletteOpen = true;
	}

	function onSettingsChange(s: AppSettings) {
		vaultName = s.vaultName ?? 'Notes';
		currentSettings = s;
		setDateFormat(s.dateFormat ?? 'long-en');
	}

	function onGlobalKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			if (settingsOpen || metaPageOpen) return;
			paletteOpen = true;
		}
		if (e.ctrlKey && e.shiftKey && !e.altKey && !e.metaKey) {
			if (e.code === 'KeyH') { e.preventDefault(); goHome(); }
			else if (e.code === 'KeyP') { e.preventDefault(); goBack(); }
			else if (e.code === 'KeyN') { e.preventDefault(); goForward(); }
			else if (e.code === 'KeyF') { e.preventDefault(); focusMode = !focusMode; }
		}
	}
</script>

<svelte:window onkeydown={onGlobalKeydown} />

{#if !loggedIn}
	<LoginPage onLogin={() => { loggedIn = true; }} {oidcError} />
{:else}

{#if confirmDialog}
	<ConfirmDialog
		message={confirmDialog.message}
		confirmLabel="Delete"
		onConfirm={confirmDialog.onConfirm}
		onCancel={() => (confirmDialog = null)}
	/>
{/if}

{#if shareModalOpen && selected}
	<ShareModal
		noteName={selected}
		onClose={() => (shareModalOpen = false)}
	/>
{/if}

{#if settingsOpen}
	<Settings
		{currentTheme}
		initialSettings={currentSettings}
		onClose={() => (settingsOpen = false)}
		onSetTheme={(id) => { currentTheme = id; applyTheme(id); }}
		onLogout={async () => { await logout(); loggedIn = false; }}
		{onSettingsChange}
	/>
{/if}

{#if paletteOpen}
	<CommandPalette
		{notes}
		{selected}
		{noteMarkdown}
		{rawView}
		{currentTheme}
		onSelect={selectNote}
		onClose={() => (paletteOpen = false)}
		onNewNote={() => {
			creatingFromPalette = true;
			sidebarOpen = true;
		}}
		onRename={() => (renaming = true)}
		onDelete={handleDelete}
		onToggleRaw={toggleRawView}
		onSetTheme={(id) => { currentTheme = id; applyTheme(id); }}
		onSettings={() => (settingsOpen = true)}
		onMediaLibrary={() => (metaPageOpen = true)}
		onShare={selected ? () => (shareModalOpen = true) : undefined}
	/>
{/if}

{#if isMobile}
	<MobileTopBar
		title={selected ?? 'Notes'}
		isLocked={effectiveLocked}
		hasNote={!!selected}
		onMenu={() => (sidebarOpen = !sidebarOpen)}
		onToggleLock={toggleLock}
		onSearch={openPalette}
	/>
{/if}

<div class="layout" class:has-topbar={isMobile}>
	<Sidebar
		{notes}
		{selected}
		{vaultName}
		mobileOpen={sidebarOpen}
		startCreating={creatingFromPalette}
		hidden={focusMode && !isMobile}
		onSelect={selectNote}
		onNew={createNote}
		onMobileClose={() => (sidebarOpen = false)}
		onCreateStarted={() => (creatingFromPalette = false)}
		onSettings={() => { sidebarOpen = false; settingsOpen = true; }}
	/>

	<main class="main">
		{#if metaPageOpen}
			<MetaPage onClose={() => (metaPageOpen = false)} onNavigate={selectNote} />
		{:else if selected}
			<NoteEditorPane
				{selected}
				{noteContent}
				{noteFrontmatter}
				{noteNames}
				saving={autoSave.saving}
				saveFailed={autoSave.saveFailed}
				{isMobile}
				lockedOverride={effectiveLocked}
				bind:renaming
				bind:focusMode
				{rawView}
				reloadKey={editorReloadKey}
				{onEdit}
				{onRawEdit}
				onToggleRaw={toggleRawView}
				{onFrontmatterChange}
				onNavigate={selectNote}
				onRenamed={handleRenamed}
				onOpenPalette={openPalette}
				focusRequest={focusEditorNonce}
			/>
		{:else if loadError}
			<div class="empty-state error-state">
				<p class="error-msg">{loadError}</p>
				<button onclick={() => (loadError = null)} class="empty-btn">Dismiss</button>
			</div>
		{:else}
			<div class="empty-state">
				<p>Select a note or create one</p>
				<button onclick={openPalette} class="empty-btn">
					{isMobile ? 'Search' : 'Ctrl+K to search'}
				</button>
			</div>
		{/if}
	</main>
</div>

<style>
	.layout {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}

	.layout.has-topbar {
		height: calc(100dvh - 48px);
		margin-top: 48px;
	}

	.main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-width: 0;
	}

	/* ── Empty state ─────────────────────────────────────── */
	.empty-state {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		color: var(--muted);
	}

	.empty-state p {
		font-size: 1rem;
		margin: 0;
	}

	.empty-btn {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.4rem 0.9rem;
		font-size: 0.85rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
	}

	.error-state { color: var(--color-danger); }
	.error-msg { font-size: 0.9rem; text-align: center; max-width: 380px; }
</style>

{/if}
