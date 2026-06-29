<script lang="ts">
	import { deleteNote, type NoteStub, type NoteMeta } from './api';
	import { emit } from './events';
	import { buildTree, flatten } from './sidebarTree';

	interface Props {
		stubs: NoteStub[];
		notes: NoteMeta[];
		maxBytes: number;
		onMaxBytesChange: (v: number) => void;
		onNavigate: (name: string) => void;
		onDeleted: (name: string) => void;
	}

	let { stubs, notes, maxBytes, onMaxBytesChange, onNavigate, onDeleted }: Props = $props();

	let selected = $state(new Set<string>());
	let confirming = $state(false);
	let deleting = $state(false);
	let showAll = $state(false);
	let openFolders = $state(new Set<string>());

	const THRESHOLDS = [
		{ label: 'All',     value: -1  },
		{ label: 'Empty',   value: 0   },
		{ label: '< 100 B', value: 100 },
		{ label: '< 500 B', value: 500 },
		{ label: '< 1 KB',  value: 1000 },
	];

	// ── All-notes tree ────────────────────────────────────────────────────────

	const visibleNotes = $derived(notes.filter(n => !n.is_template));

	const treeItems = $derived(
		showAll ? flatten(buildTree(visibleNotes), openFolders) : []
	);

	function toggleFolderOpen(path: string) {
		const next = new Set(openFolders);
		if (next.has(path)) next.delete(path); else next.add(path);
		openFolders = next;
	}

	function notesUnder(folderPath: string): string[] {
		const prefix = folderPath + '/';
		return visibleNotes.filter(n => n.name.startsWith(prefix)).map(n => n.name);
	}

	function isFolderChecked(path: string): boolean {
		const under = notesUnder(path);
		return under.length > 0 && under.every(n => selected.has(n));
	}

	function isFolderIndeterminate(path: string): boolean {
		const under = notesUnder(path);
		const selCount = under.filter(n => selected.has(n)).length;
		return selCount > 0 && selCount < under.length;
	}

	function toggleFolderSelected(path: string) {
		const under = notesUnder(path);
		const next = new Set(selected);
		if (isFolderChecked(path)) {
			under.forEach(n => next.delete(n));
		} else {
			under.forEach(n => next.add(n));
		}
		selected = next;
	}

	function noteCountUnder(folderPath: string): number {
		return notesUnder(folderPath).length;
	}

	// ── Shared selection logic ────────────────────────────────────────────────

	const currentNames = $derived(
		showAll ? visibleNotes.map(n => n.name) : stubs.map(s => s.name)
	);

	const allSelected = $derived(
		currentNames.length > 0 && currentNames.every(n => selected.has(n))
	);
	const someSelected = $derived(selected.size > 0 && !allSelected);

	function toggleAll() {
		if (allSelected) {
			selected = new Set();
		} else {
			selected = new Set(currentNames);
		}
	}

	function toggleNote(name: string) {
		const next = new Set(selected);
		if (next.has(name)) next.delete(name); else next.add(name);
		selected = next;
	}

	function selectThreshold(value: number) {
		if (value === -1) {
			showAll = true;
		} else {
			showAll = false;
			onMaxBytesChange(value);
		}
		selected = new Set();
		openFolders = new Set();
	}

	// ── Delete ────────────────────────────────────────────────────────────────

	async function deleteSelected() {
		deleting = true;
		const names = [...selected];
		for (const name of names) {
			try {
				await deleteNote(name);
				emit(document, 'notes:changed');
				onDeleted(name);
			} catch { /* skip failed */ }
		}
		selected = new Set();
		deleting = false;
		confirming = false;
	}

	// ── Stub helpers ──────────────────────────────────────────────────────────

	function formatSize(bytes: number): string {
		if (bytes === 0) return 'empty';
		if (bytes < 1000) return `${bytes} B`;
		return `${(bytes / 1000).toFixed(1)} KB`;
	}

	function stubDisplayName(stub: NoteStub): string {
		return stub.title || stub.name.split('/').pop() || stub.name;
	}
</script>

<div class="stubs-toolbar">
	<span class="stubs-label">{showAll ? 'Show:' : 'Show notes smaller than:'}</span>
	<div class="threshold-pills">
		{#each THRESHOLDS as t}
			<button
				class="pill"
				class:active={t.value === -1 ? showAll : !showAll && maxBytes === t.value}
				onclick={() => selectThreshold(t.value)}
			>{t.label}</button>
		{/each}
	</div>
	{#if selected.size > 0}
		<div class="bulk-actions">
			{#if confirming}
				<span class="confirm-label">Delete {selected.size} note{selected.size > 1 ? 's' : ''}?</span>
				<button class="confirm-btn" disabled={deleting} onclick={deleteSelected}>
					{deleting ? 'Deleting…' : 'Confirm'}
				</button>
				<button class="cancel-btn" disabled={deleting} onclick={() => (confirming = false)}>Cancel</button>
			{:else}
				<button class="delete-selected-btn" onclick={() => (confirming = true)}>
					Delete {selected.size} selected
				</button>
			{/if}
		</div>
	{/if}
</div>

{#if showAll}
	<!-- ── All-notes tree view ── -->
	{#if visibleNotes.length === 0}
		<div class="empty-state">No notes.</div>
	{:else}
		<ul class="stubs-list">
			<li class="stub-item stub-header">
				<input
					type="checkbox"
					class="stub-check"
					checked={allSelected}
					indeterminate={someSelected}
					onchange={toggleAll}
					aria-label="Select all"
				/>
				<span class="stub-header-label">Select all ({visibleNotes.length} notes)</span>
			</li>
			{#each treeItems as item (item.kind + ':' + item.path)}
				{#if item.kind === 'folder'}
					<li class="stub-item folder-item" style="padding-left: calc(1rem + {item.depth * 1.2}rem)">
						<input
							type="checkbox"
							class="stub-check"
							checked={isFolderChecked(item.path)}
							indeterminate={isFolderIndeterminate(item.path)}
							onchange={() => toggleFolderSelected(item.path)}
							aria-label="Select folder {item.label}"
						/>
						<button class="folder-toggle" onclick={() => toggleFolderOpen(item.path)}>
							<span class="folder-chevron">{item.open ? '▼' : '▶'}</span>
							<span class="folder-label">{item.label}</span>
							<span class="folder-count">{noteCountUnder(item.path)}</span>
						</button>
					</li>
				{:else}
					<li class="stub-item" class:is-selected={selected.has(item.path)} style="padding-left: calc(1rem + {item.depth * 1.2}rem)">
						<input
							type="checkbox"
							class="stub-check"
							checked={selected.has(item.path)}
							onchange={() => toggleNote(item.path)}
							aria-label="Select {item.path}"
						/>
						<button class="stub-name" onclick={() => onNavigate(item.path)}>
							{item.label}
						</button>
						{#if item.depth === 0 && item.path.includes('/')}
							<span class="stub-path">{item.path.split('/').slice(0, -1).join('/')}</span>
						{/if}
					</li>
				{/if}
			{/each}
		</ul>
	{/if}
{:else}
	<!-- ── Stub size-filter view ── -->
	{#if stubs.length === 0}
		<div class="empty-state">No notes found for this threshold.</div>
	{:else}
		<ul class="stubs-list">
			<li class="stub-item stub-header">
				<input
					type="checkbox"
					class="stub-check"
					checked={allSelected}
					indeterminate={someSelected}
					onchange={toggleAll}
					aria-label="Select all"
				/>
				<span class="stub-header-label">Select all</span>
				<span class="stub-size">Size</span>
			</li>
			{#each stubs as stub}
				<li class="stub-item" class:is-selected={selected.has(stub.name)}>
					<input
						type="checkbox"
						class="stub-check"
						checked={selected.has(stub.name)}
						onchange={() => toggleNote(stub.name)}
						aria-label="Select {stub.name}"
					/>
					<button class="stub-name" onclick={() => onNavigate(stub.name)}>
						{stubDisplayName(stub)}
					</button>
					{#if stub.name !== stubDisplayName(stub)}
						<span class="stub-path">{stub.name}</span>
					{/if}
					<span class="stub-size">{formatSize(stub.body_len)}</span>
				</li>
			{/each}
		</ul>
	{/if}
{/if}

<style>
	.stubs-toolbar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		flex-wrap: wrap;
	}

	.stubs-label {
		font-size: 0.8rem;
		color: var(--muted);
		white-space: nowrap;
	}

	.threshold-pills { display: flex; gap: 0.35rem; }

	.pill {
		padding: 0.2rem 0.65rem;
		border-radius: 999px;
		font-size: 0.75rem;
		background: var(--bg-alt, var(--sidebar-bg));
		border: 1px solid var(--border);
		color: var(--text);
		cursor: pointer;
		transition: background 0.15s, color 0.15s;
	}

	.pill.active {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}

	.bulk-actions {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-left: auto;
	}

	.confirm-label {
		font-size: 0.8rem;
		color: var(--muted);
	}

	.delete-selected-btn, .confirm-btn {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
		border: 1px solid var(--danger, #e05252);
		background: var(--danger, #e05252);
		color: #fff;
		cursor: pointer;
	}

	.delete-selected-btn:hover, .confirm-btn:hover { opacity: 0.85; }
	.confirm-btn:disabled { opacity: 0.6; cursor: default; }

	.cancel-btn {
		font-size: 0.75rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
		border: 1px solid var(--border);
		background: var(--bg-alt, var(--sidebar-bg));
		color: var(--text);
		cursor: pointer;
	}

	.stubs-list {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow-y: auto;
		flex: 1;
	}

	.stub-header {
		background: var(--bg-alt, var(--sidebar-bg));
		font-size: 0.75rem;
		color: var(--muted);
		font-weight: 600;
		border-bottom: 1px solid var(--border);
	}

	.stub-header-label { flex: 1; }

	.stub-item {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.35rem 1rem;
		border-bottom: 1px solid var(--border);
		transition: background 0.1s;
	}

	.stub-item:last-child { border-bottom: none; }
	.stub-item.is-selected { background: var(--bg-alt, var(--sidebar-bg)); }

	.stub-check { flex-shrink: 0; cursor: pointer; }

	/* ── Folder rows ─────────────────────────── */
	.folder-item {
		background: color-mix(in srgb, var(--border) 30%, transparent);
	}

	.folder-toggle {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text);
		font-family: inherit;
		font-size: 0.82rem;
		font-weight: 500;
		padding: 0;
		flex: 1;
		text-align: left;
	}

	.folder-toggle:hover { color: var(--accent); }

	.folder-chevron {
		font-size: 0.6rem;
		flex-shrink: 0;
		color: var(--muted);
	}

	.folder-label { flex: 1; }

	.folder-count {
		font-size: 0.7rem;
		color: var(--muted);
		background: var(--border);
		border-radius: 999px;
		padding: 0.05rem 0.4rem;
		flex-shrink: 0;
	}

	/* ── Note rows ───────────────────────────── */
	.stub-name {
		background: none;
		border: none;
		padding: 0;
		font-size: 0.875rem;
		color: var(--accent);
		cursor: pointer;
		text-align: left;
		flex-shrink: 0;
	}

	.stub-name:hover { text-decoration: underline; }

	.stub-path {
		font-size: 0.75rem;
		color: var(--muted);
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.stub-size {
		font-size: 0.75rem;
		color: var(--muted);
		white-space: nowrap;
		margin-left: auto;
	}

	.empty-state {
		padding: 2rem;
		text-align: center;
		color: var(--muted);
		font-size: 0.875rem;
	}
</style>
