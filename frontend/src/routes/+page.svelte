<script lang="ts">
	import { onMount } from 'svelte';
	import Sidebar from '$lib/Sidebar.svelte';
	import Editor from '$lib/Editor.svelte';
	import Backlinks from '$lib/Backlinks.svelte';
	import CommandPalette from '$lib/CommandPalette.svelte';
	import FrontmatterEditor from '$lib/FrontmatterEditor.svelte';
	import Settings from '$lib/Settings.svelte';
	import MetaPage from '$lib/MetaPage.svelte';
	import LoginPage from '$lib/LoginPage.svelte';
	import TableOfContents, { type Heading } from '$lib/TableOfContents.svelte';
	import {
		listNotes,
		getNote,
		saveNote,
		renameNote,
		deleteNote,
		getSettings,
		serializeFrontmatter,
		session,
		logout,
		type NoteMeta,
		type Frontmatter,
	} from '$lib/api';
	import { loadTheme, applyTheme, type ThemeId } from '$lib/theme';
	import { applySettings, DEFAULT, type AppSettings } from '$lib/settings';
	import { setDateFormat } from '$lib/slashCommands';
	import { emit, on } from '$lib/events';
	import { createNavigation } from '$lib/navigation.svelte';
	import TitleBar from '$lib/TitleBar.svelte';
	import MobileTopBar from '$lib/MobileTopBar.svelte';

	let notes = $state<NoteMeta[]>([]);
	let selected = $state<string | null>(null);
	let noteContent = $state('');
	let noteFrontmatter = $state<Frontmatter>({});
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let saving = $state(false);
	let saveFailed = $state(false);
	let paletteOpen = $state(false);
	let sidebarOpen = $state(false);   // mobile drawer state
	let renaming = $state(false);
	let renameValue = $state('');
	let renameError = $state('');
	let isMobile = $state(false);
	let creatingFromPalette = $state(false);  // true = open new-note input in sidebar
	let currentTheme = $state<ThemeId>('default');
	let vaultName = $state('Notes');
	let loggedIn = $state(session.exists());
	let settingsOpen = $state(false);
	let metaPageOpen = $state(false);
	let currentSettings = $state<AppSettings>({ ...DEFAULT });
	const nav = createNavigation();
	let focusMode = $state(false);


	const noteMarkdown = $derived(serializeFrontmatter(noteFrontmatter) + noteContent);

	const headings = $derived<Heading[]>(
		Array.from(noteContent.matchAll(/^(#{1,6})\s+(.+?)(?:\s+#+\s*)?$/gm)).map((m) => ({
			level: m[1].length,
			text: m[2].trim(),
		}))
	);

	const showToc = $derived(
		headings.length >= 2 && noteFrontmatter.toc !== false
	);

	const noteNames = $derived(notes.map(n => n.name));
	const isIndex = $derived(noteFrontmatter.type === 'index');
	const isLocked = $derived(noteFrontmatter.locked === true);

	function disableToc() {
		onFrontmatterChange({ ...noteFrontmatter, toc: false });
	}

	// One-time setup: auth expiry + media query — neither depends on reactive state.
	onMount(() => {
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
	// The handler reads `notes` lazily at event-fire time, so it always sees the current list.
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
		if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
		const note = await getNote(name);
		noteFrontmatter = (note.frontmatter ?? {}) as Frontmatter;
		noteContent = note.content;
		selected = name;
		sidebarOpen = false;
		metaPageOpen = false;
		if (pushHistory) nav.push(name);
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
		await saveNote(name, '');
		notes = [...notes, { name, pinned: false }];
		await selectNote(name);
	}

	function triggerSave(name: string, fm: Frontmatter, body: string) {
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(async () => {
			saving = true;
			saveFailed = false;
			try {
				const fullContent = serializeFrontmatter(fm) + body;
				await saveNote(name, fullContent);
				// Derive metadata locally — avoids a full round-trip on every keystroke
				notes = notes.map(n =>
					n.name === name
						? { ...n, pinned: fm.pinned === true, is_index: fm.type === 'index', is_template: fm.type === 'template' }
						: n
				);
			} catch {
				saveFailed = true;
			} finally {
				saving = false;
			}
		}, 800);
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
		triggerSave(selected, noteFrontmatter, markdown);
	}

	function onFrontmatterChange(fm: Frontmatter) {
		if (!selected) return;
		noteFrontmatter = fm;
		triggerSave(selected, fm, noteContent);
	}

	function startRename() {
		if (!selected) return;
		renameValue = selected;
		renameError = '';
		renaming = true;
	}

	async function confirmRename() {
		if (!selected || !renaming) return;
		renaming = false;
		const newName = renameValue.trim();
		if (!newName || newName === selected) return;
		if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
		const oldName = selected;
		try {
			await renameNote(oldName, newName);
			notes = notes.map(n => n.name === oldName ? { ...n, name: newName } : n);
			selected = newName;
			emit(document, 'notes:changed');
		} catch (e: unknown) {
			renameError = e instanceof Error ? e.message : 'Rename failed';
			renaming = true;
		}
	}

	function cancelRename() {
		renaming = false;
		renameError = '';
	}

	async function handleDelete() {
		if (!selected) return;
		if (!confirm(`Delete "${selected}"? This action cannot be undone.`)) return;
		const name = selected;
		selected = null;
		noteContent = '';
		noteFrontmatter = {};
		await deleteNote(name);
		notes = notes.filter(n => n.name !== name);
		document.dispatchEvent(new CustomEvent('notes:changed'));
	}

	function toggleLock() {
		const fm = { ...noteFrontmatter };
		if (isLocked) {
			delete fm.locked;
		} else {
			fm.locked = true;
		}
		onFrontmatterChange(fm);
	}

	function openPalette() {
		paletteOpen = true;
	}

	function onSettingsChange(s: AppSettings) {
		vaultName = s.vaultName ?? 'Notes';
		currentSettings = s;
		setDateFormat(s.dateFormat ?? 'long-en');
	}

	function openSettings() {
		settingsOpen = true;
	}

	function openMetaPage() {
		metaPageOpen = true;
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
	<LoginPage onLogin={() => { loggedIn = true; }} />
{:else}

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
		{currentTheme}
		onSelect={selectNote}
		onClose={() => (paletteOpen = false)}
		onNewNote={() => {
			creatingFromPalette = true;
			sidebarOpen = true;
		}}
		onRename={startRename}
		onDelete={handleDelete}
		onSetTheme={(id) => { currentTheme = id; applyTheme(id); }}
		onSettings={openSettings}
		onMediaLibrary={openMetaPage}
	/>
{/if}

{#if isMobile}
	<MobileTopBar
		title={selected ?? 'Notes'}
		{isLocked}
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
	/>

	<main class="main" class:focus-mode={focusMode}>
		{#if metaPageOpen}
			<MetaPage onClose={() => (metaPageOpen = false)} />
		{:else if selected}
			<TitleBar
				{selected}
				{saving}
				{saveFailed}
				{isLocked}
				{focusMode}
				{isMobile}
				bind:renaming
				bind:renameValue
				renameError={renameError}
				onStartRename={startRename}
				onConfirmRename={confirmRename}
				onCancelRename={cancelRename}
				onToggleLock={toggleLock}
				onToggleFocus={() => (focusMode = !focusMode)}
				onOpenPalette={openPalette}
			/>

			{#if showToc && !focusMode}
				<TableOfContents {headings} onDisable={disableToc} />
			{/if}
			<div class="editor-area" class:focus-mode={focusMode}>
				{#key selected}
					<FrontmatterEditor
						frontmatter={noteFrontmatter}
						onChange={onFrontmatterChange}
					/>
					<Editor {noteContent} {noteNames} {onEdit} {isIndex} {isLocked} />
				{/key}
			</div>
			{#if !focusMode}
				<Backlinks note={selected} onNavigate={selectNote} />
			{/if}
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


	/* ── Focus mode ─────────────────────────────────────────────── */
	.editor-area {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0; /* allows flex child to shrink and scroll correctly */
		min-width: 0;
	}

	.editor-area.focus-mode {
		max-width: clamp(760px, 75vw, 1200px);
		width: 100%;
		margin: 0 auto;
		overflow: hidden;
	}

	.editor-area.focus-mode :global(.editor-wrap) {
		scrollbar-width: none;
		-ms-overflow-style: none;
	}

	.editor-area.focus-mode :global(.editor-wrap::-webkit-scrollbar) {
		display: none;
	}

	/* ── Empty state ─────────────────────────────────────────── */
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
</style>

{/if}
