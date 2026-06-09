<script lang="ts">
	import { untrack } from 'svelte';
	import type { NoteMeta, VaultInfo } from './api';
	import { buildTree, flatten, type DisplayItem } from './sidebarTree';
	import { storage } from './storage';
	import { escapeHtml } from './utils';
	import VaultSwitcher from './VaultSwitcher.svelte';

	interface Props {
		notes: NoteMeta[];
		selected: string | null;
		vaults?: VaultInfo[];
		mobileOpen?: boolean;
		startCreating?: boolean;
		hidden?: boolean;
		onSelect: (name: string) => void;
		onNew: (name: string) => void;
		onMobileClose?: () => void;
		onCreateStarted?: () => void;
		onSettings?: () => void;
		onVaultSwitch?: (slug: string) => void;
		onVaultCreated?: (vault: VaultInfo) => void;
		onVaultDeleted?: (slug: string) => void;
	}

	let { notes, selected, vaults = [], mobileOpen = false, startCreating = false, hidden = false, onSelect, onNew, onMobileClose, onCreateStarted, onSettings, onVaultSwitch, onVaultCreated, onVaultDeleted }: Props = $props();

	$effect(() => {
		if (startCreating) {
			creating = true;
			onCreateStarted?.();
		}
	});

	let newName = $state('');
	let creating = $state(false);
	let filterQuery = $state('');
	let filterInput = $state<HTMLInputElement | null>(null);
	let collapsed = $state(storage.sidebar.getCollapsed());

	function toggleCollapse() {
		if (mobileOpen) { onMobileClose?.(); return; }
		collapsed = !collapsed;
		storage.sidebar.setCollapsed(collapsed);
	}

	// ── Resize ────────────────────────────────────────────────────
	const MIN_WIDTH = 160;
	const MAX_WIDTH = 480;
	const DEFAULT_WIDTH = 240;

	let sidebarWidth = $state(storage.sidebar.getWidth(DEFAULT_WIDTH));
	let dragging = $state(false);
	let dragStartX = 0;
	let dragStartWidth = 0;

	function onHandleMouseDown(e: MouseEvent) {
		e.preventDefault();
		dragging = true;
		dragStartX = e.clientX;
		dragStartWidth = sidebarWidth;
	}

	function onMouseMove(e: MouseEvent) {
		if (!dragging) return;
		sidebarWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, dragStartWidth + e.clientX - dragStartX));
	}

	function onMouseUp() {
		if (!dragging) return;
		dragging = false;
		storage.sidebar.setWidth(sidebarWidth);
	}

	let openFolders = $state<Set<string>>(new Set());

	function toggleFolder(path: string) {
		const next = new Set(openFolders);
		if (next.has(path)) next.delete(path);
		else next.add(path);
		openFolders = next;
	}

	$effect(() => {
		const sel = selected;
		if (!sel) return;
		const parts = sel.split('/');
		if (parts.length <= 1) return;
		const next = new Set(untrack(() => openFolders));
		for (let i = 1; i < parts.length; i++) next.add(parts.slice(0, i).join('/'));
		openFolders = next;
	});

	let indexNotes = $derived(notes.filter(n => n.is_index).sort((a, b) => a.name.localeCompare(b.name)));
	let tree = $derived(buildTree(notes.filter(n => !n.is_template && !n.is_index)));
	let items = $derived<DisplayItem[]>(flatten(tree, openFolders));

	const q = $derived(filterQuery.trim().toLowerCase());
	let filteredNotes = $derived(
		q
			? notes
				.filter(n => !n.is_template && n.name.toLowerCase().includes(q))
				.sort((a, b) => a.name.localeCompare(b.name))
			: null
	);

	function highlight(text: string, query: string): string {
		const idx = text.toLowerCase().indexOf(query);
		if (idx === -1) return escapeHtml(text);
		return escapeHtml(text.slice(0, idx)) + '<mark>' + escapeHtml(text.slice(idx, idx + query.length)) + '</mark>' + escapeHtml(text.slice(idx + query.length));
	}

	function focus(el: HTMLElement) { el.focus(); }

	function submitNew() {
		const trimmed = newName.trim();
		if (!trimmed) return;
		onNew(trimmed);
		newName = '';
		creating = false;
	}
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

{#if mobileOpen}
	<button class="mobile-backdrop" onclick={onMobileClose} aria-label="Close menu"></button>
{/if}

<aside
	class="sidebar"
	class:mobile-open={mobileOpen}
	class:collapsed
	class:dragging
	class:hidden
	style={collapsed || hidden ? '' : `width: ${sidebarWidth}px`}
>
	<div class="sidebar-head">
		{#if !collapsed}
			<div class="vault-area">
				{#if vaults.length > 0 && (onVaultSwitch || onVaultCreated || onVaultDeleted)}
					<VaultSwitcher
						{vaults}
						onSwitch={(slug) => onVaultSwitch?.(slug)}
						onCreated={(vault) => onVaultCreated?.(vault)}
						onDeleted={(slug) => onVaultDeleted?.(slug)}
					/>
				{:else}
					<span class="vault-name">{vaults.find(v => v.active)?.name ?? 'Notes'}</span>
				{/if}
			</div>
			<div class="head-actions">
				<button
					onclick={() => (creating = !creating)}
					class="head-btn"
					title="New note"
					aria-label="New note"
				>
					<svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
						<path d="M7 1v12M1 7h12" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
					</svg>
				</button>
				<button
					onclick={toggleCollapse}
					class="head-btn"
					title="Collapse sidebar"
					aria-label="Collapse sidebar"
				>
					<svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
						<path d="M9 2L4 7l5 5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
			</div>
		{:else}
			<button
				onclick={toggleCollapse}
				class="head-btn expand-btn"
				title="Expand sidebar"
				aria-label="Expand sidebar"
			>
				<svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
					<path d="M5 2l5 5-5 5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</button>
		{/if}
	</div>

	{#if !collapsed}
		{#if creating}
			<form
				onsubmit={(e) => { e.preventDefault(); submitNew(); }}
				class="create-form"
			>
				<input
					bind:value={newName}
					placeholder="name or folder/name"
					use:focus
					onkeydown={(e) => e.key === 'Escape' && (creating = false)}
					class="create-input"
				/>
			</form>
		{/if}

		<div class="filter-bar">
			<svg class="filter-icon" width="12" height="12" viewBox="0 0 14 14" fill="none" aria-hidden="true">
				<circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5"/>
				<path d="M10 10l2.5 2.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
			<input
				bind:this={filterInput}
				bind:value={filterQuery}
				placeholder="Filter…"
				class="filter-input"
				aria-label="Filter notes"
				onkeydown={(e) => {
					if (e.key === 'Escape') { filterQuery = ''; e.preventDefault(); }
					if (e.key === 'Enter' && filteredNotes && filteredNotes.length > 0) {
						onSelect(filteredNotes[0].name);
						filterQuery = '';
					}
				}}
			/>
			{#if filterQuery}
				<button class="filter-clear" onclick={() => { filterQuery = ''; filterInput?.focus(); }} aria-label="Clear filter">
					<svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
						<path d="M2 2l6 6M8 2L2 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				</button>
			{/if}
		</div>

		{#if !filterQuery}
			{#if indexNotes.length > 0}
				<div class="index-section">
					{#each indexNotes as note (note.name)}
						<button
							onclick={() => onSelect(note.name)}
							class="index-btn"
							class:active={selected === note.name}
							title={note.name}
						>
							<svg class="index-icon" width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
								<rect x="1" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
								<rect x="7" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
								<rect x="1" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
								<rect x="7" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
							</svg>
							{note.name.split('/').pop()}
						</button>
					{/each}
				</div>
				<div class="index-divider"></div>
			{/if}

			<ul class="note-list">
				{#each items as item (item.kind + ':' + item.path)}
					<li>
						{#if item.kind === 'folder'}
							<button
								onclick={() => toggleFolder(item.path)}
								class="folder-btn"
								style="padding-left: calc(1rem + {item.depth * 0.9}rem);"
							>
								<span class="folder-chevron">{item.open ? '▼' : '▶'}</span>
								{item.label}
							</button>
						{:else}
							<button
								onclick={() => onSelect(item.path)}
								class="note-btn"
								class:active={selected === item.path}
								style="padding-left: calc(1rem + {item.depth * 0.9}rem);"
							>
								{#if item.pinned}<span class="pin-dot" aria-hidden="true"></span>{/if}
								{item.label}
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{:else}
			<ul class="note-list">
				{#if filteredNotes && filteredNotes.length > 0}
					{#each filteredNotes as note (note.name)}
						{@const label = note.name.split('/').pop()!}
						{@const folder = note.name.includes('/') ? note.name.split('/').slice(0, -1).join('/') : ''}
						<li>
							<button
								onclick={() => { onSelect(note.name); filterQuery = ''; }}
								class="note-btn filter-result"
								class:active={selected === note.name}
							>
								<span class="filter-label">{@html highlight(label, q)}</span>
								{#if folder}<span class="filter-path">{folder.split('/').pop()}</span>{/if}
							</button>
						</li>
					{/each}
				{:else}
					<li class="filter-empty">No results</li>
				{/if}
			</ul>
		{/if}

		{#if onSettings}
			<div class="sidebar-footer">
				<button class="footer-btn" onclick={() => onSettings?.()} title="Settings" aria-label="Settings">
					<svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
						<circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8"/>
						<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
					<span>Settings</span>
				</button>
			</div>
		{/if}
	{/if}

	{#if !collapsed}
		<button
			class="resize-handle"
			onmousedown={onHandleMouseDown}
			onkeydown={(e) => {
				const step = e.shiftKey ? 20 : 8;
				if (e.key === 'ArrowRight') { sidebarWidth = Math.min(MAX_WIDTH, sidebarWidth + step); storage.sidebar.setWidth(sidebarWidth); }
				if (e.key === 'ArrowLeft')  { sidebarWidth = Math.max(MIN_WIDTH, sidebarWidth - step); storage.sidebar.setWidth(sidebarWidth); }
			}}
			aria-label="Resize sidebar"
		></button>
	{/if}
</aside>

<style>
	.sidebar {
		width: 240px;
		min-width: 0;
		height: 100vh;
		background: var(--sidebar-bg);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		flex-shrink: 0;
		transition: width 200ms ease;
		position: relative;
	}

	.sidebar.collapsed {
		width: 44px;
	}

	.sidebar.dragging {
		transition: none;
		user-select: none;
	}

	.sidebar.hidden {
		width: 0 !important;
		border-right: none;
		overflow: hidden;
	}

	/* ── Resize handle ───────────────────────────────────────── */
	.resize-handle {
		position: absolute;
		top: 0;
		right: 0;
		width: 4px;
		height: 100%;
		cursor: ew-resize;
		z-index: 10;
		transition: background 150ms;
	}

	.resize-handle:hover,
	.dragging .resize-handle {
		background: color-mix(in srgb, var(--accent) 40%, transparent);
	}

	/* ── Header ──────────────────────────────────────────── */
	.sidebar-head {
		padding: 0 0.5rem;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		gap: 0.1rem;
		flex-shrink: 0;
		min-height: 44px;
	}

	.vault-area {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
	}

	.vault-name {
		flex: 1;
		font-size: 0.7rem;
		font-weight: 600;
		letter-spacing: 0.07em;
		text-transform: uppercase;
		color: var(--muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		padding-left: 0.35rem;
	}

	.head-actions {
		display: flex;
		align-items: center;
	}

	.head-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0.35rem;
		border-radius: 5px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 80ms, color 80ms;
		flex-shrink: 0;
	}

	.head-btn:hover {
		background: var(--border);
		color: var(--text);
	}

	.expand-btn {
		margin: 0 auto;
	}

	/* ── Create form ─────────────────────────────────────── */
	.create-form {
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.create-input {
		width: 100%;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 0.3rem 0.5rem;
		font-size: 0.85rem;
		color: var(--text);
		outline: none;
		font-family: inherit;
		box-sizing: border-box;
	}

	.create-input:focus {
		border-color: var(--accent);
	}

	/* ── Filter bar ─────────────────────────────────────── */
	.filter-bar {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.35rem 0.6rem 0.35rem 0.75rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.filter-icon {
		color: var(--muted);
		flex-shrink: 0;
		opacity: 0.6;
	}

	.filter-input {
		flex: 1;
		background: none;
		border: none;
		outline: none;
		font: inherit;
		font-size: 0.85rem;
		color: var(--text);
		min-width: 0;
	}

	.filter-input::placeholder {
		color: var(--muted);
		opacity: 0.6;
	}

	.filter-clear {
		color: var(--muted);
		padding: 2px;
		border-radius: 3px;
		display: flex;
		align-items: center;
		flex-shrink: 0;
		opacity: 0.6;
		transition: opacity 80ms;
	}

	.filter-clear:hover {
		opacity: 1;
	}

	/* ── Filter results ──────────────────────────────────── */
	.filter-result {
		padding-left: 1rem !important;
		gap: 0;
	}

	.filter-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	:global(.filter-label mark) {
		background: color-mix(in srgb, var(--accent) 25%, transparent);
		color: inherit;
		border-radius: 2px;
	}

	.filter-path {
		font-size: 0.72rem;
		color: var(--muted);
		white-space: nowrap;
		flex-shrink: 0;
		padding-left: 0.4rem;
	}

	.filter-empty {
		padding: 0.5rem 1rem;
		font-size: 0.82rem;
		color: var(--muted);
		font-style: italic;
	}

	/* ── Index pages section ─────────────────────────────── */
	.index-section {
		padding: 0.4rem 0 0.2rem;
		flex-shrink: 0;
	}

	.index-btn {
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.38rem 1rem 0.38rem 0.75rem;
		font-size: 0.88rem;
		color: var(--text);
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: inherit;
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-weight: 500;
	}

	.index-btn:hover {
		background: var(--border);
	}

	.index-btn.active {
		background: var(--border);
	}

	.index-icon {
		flex-shrink: 0;
		opacity: 0.6;
		color: var(--accent);
	}

	.index-divider {
		height: 1px;
		background: var(--border);
		margin: 0.2rem 0.75rem 0;
	}

	/* ── Note list ───────────────────────────────────────── */
	.note-list {
		margin: 0;
		padding: 0.4rem 0;
		list-style: none;
		overflow-y: auto;
		flex: 1;
	}

	.folder-btn {
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding-top: 0.3rem;
		padding-bottom: 0.3rem;
		padding-right: 1rem;
		font-size: 0.78rem;
		font-variant: small-caps;
		color: var(--muted);
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 0.3rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: inherit;
	}

	.folder-btn:hover {
		color: var(--text);
	}

	.folder-chevron {
		font-size: 0.6rem;
		flex-shrink: 0;
	}

	.note-btn {
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding-top: 0.42rem;
		padding-bottom: 0.42rem;
		padding-right: 1rem;
		font-size: 0.88rem;
		color: var(--text);
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: inherit;
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.note-btn:hover {
		background: var(--border);
	}

	.note-btn.active {
		background: var(--border);
	}

	.pin-dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--accent);
		flex-shrink: 0;
	}

	/* ── Footer (settings) ───────────────────────────────── */
	.sidebar-footer {
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		padding: 0.25rem;
	}

	.footer-btn {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.55rem;
		padding: 0.5rem 0.6rem;
		background: none;
		border: none;
		border-radius: 6px;
		color: var(--text);
		cursor: pointer;
		font-family: inherit;
		font-size: 0.88rem;
		transition: background 80ms;
	}

	.footer-btn:hover {
		background: var(--border);
	}

	.footer-btn svg {
		color: var(--muted);
		flex-shrink: 0;
	}

	/* ── Mobile ──────────────────────────────────────────── */
	@media (max-width: 640px) {
		.sidebar {
			position: fixed;
			left: 0;
			top: 0;
			bottom: 0;
			z-index: 150;
			width: 240px;
			transform: translateX(-100%);
			transition: transform 220ms ease;
			box-shadow: 4px 0 32px rgba(0, 0, 0, 0.18);
			height: 100dvh;
		}

		.sidebar.collapsed {
			width: 240px;
		}

		.sidebar.mobile-open {
			transform: translateX(0);
		}

		.mobile-backdrop {
			position: fixed;
			inset: 0;
			background: rgba(0, 0, 0, 0.4);
			z-index: 149;
			border: none;
			cursor: default;
		}

		.footer-btn {
			padding: 0.7rem 0.6rem;
		}

		.sidebar-footer {
			padding-bottom: calc(0.25rem + env(safe-area-inset-bottom, 0));
		}
	}
</style>
