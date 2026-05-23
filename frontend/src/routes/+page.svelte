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
		serializeFrontmatter,
		session,
		logout,
		type NoteMeta,
		type Frontmatter,
	} from '$lib/api';
	import { loadTheme, applyTheme, type ThemeId } from '$lib/theme';
	import { applySettings, DEFAULT, type AppSettings } from '$lib/settings';
	import { setDateFormat } from '$lib/slashCommands';
	import { getSettings } from '$lib/api';
	import TitleBar from '$lib/TitleBar.svelte';

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
	let navHistory = $state<string[]>([]);
	let navIndex = $state(-1);
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

	const isIndex = $derived(noteFrontmatter.type === 'index');
	const isLocked = $derived(noteFrontmatter.locked === true);

	function disableToc() {
		onFrontmatterChange({ ...noteFrontmatter, toc: false });
	}

	onMount(() => {
		const onAuthExpired = () => { loggedIn = false; };
		window.addEventListener('auth:expired', onAuthExpired);
		return () => window.removeEventListener('auth:expired', onAuthExpired);
	});

	// Runs immediately if already logged in, or re-runs after login.
	// onMount alone is not enough: when a user logs in, onMount has already
	// finished and the wiki-navigate listener (and other setup) would never fire.
	$effect(() => {
		if (!loggedIn) return;

		const theme = loadTheme();
		currentTheme = theme;
		applyTheme(theme);

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

		const handler = (e: Event) => {
			const target = (e as CustomEvent<string>).detail;
			if (!target) return;
			const exact = notes.find(n => n.name === target);
			const resolved = exact
				?? notes.find(n => n.name.split('/').pop() === target)
				?? notes.find(n => n.name.toLowerCase().split('/').pop() === target.toLowerCase());
			selectNote(resolved?.name ?? target).catch(() => {});
		};
		document.addEventListener('wiki-navigate', handler);

		const mq = window.matchMedia('(max-width: 640px)');
		isMobile = mq.matches;
		const mqHandler = (e: MediaQueryListEvent) => { isMobile = e.matches; };
		mq.addEventListener('change', mqHandler);

		return () => {
			document.removeEventListener('wiki-navigate', handler);
			mq.removeEventListener('change', mqHandler);
		};
	});

	async function selectNote(name: string, pushHistory = true) {
		if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
		const note = await getNote(name);
		noteFrontmatter = (note.frontmatter ?? {}) as Frontmatter;
		noteContent = note.content;
		selected = name;
		sidebarOpen = false;
		metaPageOpen = false;
		if (pushHistory) {
			navHistory = [...navHistory.slice(0, navIndex + 1), name];
			navIndex = navHistory.length - 1;
		}
	}

	function goBack() {
		if (navIndex <= 0) return;
		const idx = navIndex - 1;
		selectNote(navHistory[idx], false).then(() => { navIndex = idx; });
	}

	function goForward() {
		if (navIndex >= navHistory.length - 1) return;
		const idx = navIndex + 1;
		selectNote(navHistory[idx], false).then(() => { navIndex = idx; });
	}

	function goHome() {
		const home = currentSettings.homePage?.trim();
		if (!home) return;
		selectNote(home);
	}

	async function createNote(name: string) {
		await saveNote(name, '');
		notes = await listNotes();
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
		try {
			await renameNote(selected, newName);
			notes = await listNotes();
			selected = newName;
			document.dispatchEvent(new CustomEvent('notes:changed'));
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
		notes = await listNotes();
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
		onSettingsChange={(s: AppSettings) => { vaultName = s.vaultName ?? 'Notes'; currentSettings = s; setDateFormat(s.dateFormat ?? 'long-en'); }}
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

<!-- ── Mobile top bar ───────────────────────────────────────── -->
{#if isMobile}
	<header class="mobile-topbar">
		<button class="topbar-btn" onclick={() => (sidebarOpen = !sidebarOpen)} aria-label="Menu">
			<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
				<path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
		<span class="topbar-title">{selected ?? 'Notes'}</span>
		{#if selected}
			<button
				class="topbar-btn"
				class:topbar-locked={isLocked}
				onclick={toggleLock}
				aria-label={isLocked ? 'Unlock note' : 'Lock note'}
			>
				{#if isLocked}
					<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
						<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
						<path d="M7 9V6a3 3 0 0 1 6 0v3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				{:else}
					<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
						<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
						<path d="M7 9V6a3 3 0 0 1 6 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				{/if}
			</button>
		{/if}
		<button class="topbar-btn" onclick={openPalette} aria-label="Search">
			<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
				<circle cx="8.5" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.5"/>
				<path d="M13.5 13.5L17 17" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
	</header>
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
					<Editor {noteContent} noteNames={notes.map(n => n.name)} {onEdit} {isIndex} {isLocked} />
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


	/* ── Mobile top bar ──────────────────────────────────────── */
	.mobile-topbar {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		height: 48px;
		background: var(--sidebar-bg);
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		padding: 0 0.5rem;
		z-index: 100;
		gap: 0.25rem;
	}

	.topbar-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text);
		padding: 0.4rem;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.topbar-btn svg {
		width: 20px;
		height: 20px;
	}

	.topbar-btn:active {
		background: var(--border);
	}

	.topbar-locked {
		color: var(--color-warning);
	}

	.topbar-title {
		flex: 1;
		font-size: 1.05rem;
		font-weight: 600;
		letter-spacing: -0.02em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		text-align: center;
		padding: 0 0.25rem;
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
