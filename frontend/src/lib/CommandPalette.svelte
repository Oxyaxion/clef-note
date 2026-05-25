<script lang="ts">
	import { searchContent, queryNotes, listTags, uploadAsset, type NoteMeta, type SearchResult, type NoteQueryResult, type TagCount } from './api';
	import { THEMES, type ThemeId } from './theme';
	import { escapeHtml } from './utils';
	import { emit } from './events';
	import NoteIcon from './NoteIcon.svelte';
	import {
		copyMarkdown, copyMarkdownClean, copyHtml,
		downloadMd, downloadMdClean,
		printNote, shareNote, canShare,
	} from './noteExport';

	interface Command {
		id: string;
		label: string;
		hint?: string;
		icon: string;
		action: () => void;
	}

	interface Props {
		notes: NoteMeta[];
		selected: string | null;
		noteMarkdown?: string;
		currentTheme?: ThemeId;
		onSelect: (name: string) => void;
		onClose: () => void;
		onNewNote: () => void;
		onRename: () => void;
		onDelete: () => void;
		onSetTheme: (id: ThemeId) => void;
		onSettings: () => void;
		onMediaLibrary: () => void;
	}

	let { notes, selected, noteMarkdown = '', currentTheme = 'default', onSelect, onClose, onNewNote, onRename, onDelete, onSetTheme, onSettings, onMediaLibrary }: Props = $props();

	let query = $state('');
	let selectedIndex = $state(0);
	let contentResults = $state<SearchResult[]>([]);
	let dslResults = $state<NoteQueryResult[]>([]);
	let dslTagResults = $state<TagCount[]>([]);
	let contentSearchTimer: ReturnType<typeof setTimeout> | null = null;
	let dslSearchTimer: ReturnType<typeof setTimeout> | null = null;
	let contentCtrl: AbortController | null = null;
	let dslCtrl: AbortController | null = null;

	const isCommandMode = $derived(query.startsWith('>'));
	const isQueryMode = $derived(query.startsWith('?'));
	const commandQuery = $derived(isCommandMode ? query.slice(1).trimStart() : '');
	const dslQuery = $derived(isQueryMode ? query.slice(1).trimStart() : '');

	// Stable commands — rebuilt only when themes or core actions change (not on note switch)
	const baseCommands = $derived<Command[]>([
		{
			id: 'settings',
			label: 'Settings',
			icon: '⚙',
			action: () => { onClose(); onSettings(); },
		},
		{
			id: 'media-library',
			label: 'Media library',
			icon: '⊟',
			action: () => { onClose(); onMediaLibrary(); },
		},
		{
			id: 'new',
			label: 'New note',
			icon: '＋',
			action: () => { onClose(); onNewNote(); },
		},
		// Themes — depend on currentTheme for the active indicator
		...THEMES.map((t) => ({
			id: `theme-${t.id}`,
			label: `Theme: ${t.label}`,
			icon: currentTheme === t.id ? '●' : '○',
			action: () => { onSetTheme(t.id); onClose(); },
		})),
	]);

	// Note-context commands — rebuilt only when selected note changes
	const noteCommands = $derived<Command[]>(selected ? [
		{
			id: 'rename',
			label: 'Rename',
			hint: selected,
			icon: '✎',
			action: () => { onClose(); onRename(); },
		},
		{
			id: 'delete',
			label: 'Delete',
			hint: selected,
			icon: '✕',
			action: () => { onClose(); onDelete(); },
		},
		{
			id: 'upload-image',
			label: 'Upload image',
			icon: '↑',
			action: () => {
				onClose();
				const input = document.createElement('input');
				input.type = 'file';
				input.accept = 'image/*';
				input.onchange = async () => {
					const file = input.files?.[0];
					if (!file) return;
					try {
						const url = await uploadAsset(file);
						emit(document, 'insert-image', url);
					} catch (err) {
						console.error('Image upload failed', err);
					}
				};
				input.click();
			},
		},
		{
			id: 'copy-md',
			label: 'Copy Markdown',
			icon: '⎘',
			action: async () => { await copyMarkdown(noteMarkdown); onClose(); },
		},
		{
			id: 'copy-html',
			label: 'Copy HTML',
			icon: '⎘',
			action: async () => { await copyHtml(noteMarkdown); onClose(); },
		},
		{
			id: 'download-md',
			label: 'Download .md',
			icon: '↓',
			action: () => { downloadMd(selected, noteMarkdown); onClose(); },
		},
		{
			id: 'copy-md-clean',
			label: 'Copy Markdown (no queries)',
			icon: '⎘',
			action: async () => { await copyMarkdownClean(noteMarkdown); onClose(); },
		},
		{
			id: 'download-md-clean',
			label: 'Download .md (no queries)',
			icon: '↓',
			action: () => { downloadMdClean(selected, noteMarkdown); onClose(); },
		},
		{
			id: 'print',
			label: 'Print / PDF',
			icon: '⎙',
			action: () => { onClose(); printNote(); },
		},
		...(canShare ? [{
			id: 'share',
			label: 'Share',
			icon: '↗',
			action: async () => { await shareNote(selected, noteMarkdown); onClose(); },
		}] : []),
	] : []);

	const commands = $derived<Command[]>([...baseCommands, ...noteCommands]);

	const filteredCommands = $derived(
		commandQuery.trim() === ''
			? commands
			: commands.filter((c) => c.label.toLowerCase().includes(commandQuery.toLowerCase()))
	);

	const titleResults = $derived(
		isCommandMode || isQueryMode || query.trim() === ''
			? []
			: notes
					.filter((n) => n.name.toLowerCase().includes(query.toLowerCase()))
					.slice(0, 8)
	);

	// Combined list for keyboard navigation
	type ResultItem =
		| { kind: 'note'; name: string; snippet?: string; meta?: NoteQueryResult }
		| { kind: 'command'; cmd: Command }
		| { kind: 'tag'; tag: TagCount };

	const allItems = $derived<ResultItem[]>(
		isCommandMode
			? filteredCommands.map((c) => ({ kind: 'command' as const, cmd: c }))
			: isQueryMode
				? dslTagResults.length > 0
					? dslTagResults.map((t) => ({ kind: 'tag' as const, tag: t }))
					: dslResults.map((r) => ({ kind: 'note' as const, name: r.name, meta: r }))
				: [
						...titleResults.map((n) => ({ kind: 'note' as const, name: n.name })),
						...contentResults
							.filter((r) => !titleResults.some((t) => t.name === r.name))
							.map((r) => ({ kind: 'note' as const, name: r.name, snippet: r.snippet })),
				  ]
	);

	$effect(() => {
		allItems;
		selectedIndex = 0;
	});

	// Debounced content search — abort previous request when query changes
	$effect(() => {
		const q = query;
		if (isCommandMode || isQueryMode || q.trim().length < 2) {
			contentCtrl?.abort();
			contentCtrl = null;
			contentResults = [];
			return;
		}
		if (contentSearchTimer) clearTimeout(contentSearchTimer);
		contentSearchTimer = setTimeout(() => {
			contentCtrl?.abort();
			const ctrl = new AbortController();
			contentCtrl = ctrl;
			searchContent(q, ctrl.signal)
				.then((r) => { contentResults = r; })
				.catch((e) => { if (!(e instanceof DOMException && e.name === 'AbortError')) contentResults = []; });
		}, 300);
	});

	// Debounced DSL query — abort previous request when query changes
	$effect(() => {
		const q = dslQuery;
		if (!isQueryMode) {
			dslCtrl?.abort();
			dslCtrl = null;
			dslResults = [];
			dslTagResults = [];
			return;
		}
		if (q.trim() === '#') {
			if (dslSearchTimer) clearTimeout(dslSearchTimer);
			dslSearchTimer = setTimeout(() => {
				dslCtrl?.abort();
				const ctrl = new AbortController();
				dslCtrl = ctrl;
				listTags(ctrl.signal)
					.then((r) => { dslTagResults = r; dslResults = []; })
					.catch((e) => { if (!(e instanceof DOMException && e.name === 'AbortError')) dslTagResults = []; });
			}, 150);
			return;
		}
		dslTagResults = [];
		if (q.trim().length < 2) {
			dslResults = [];
			return;
		}
		if (dslSearchTimer) clearTimeout(dslSearchTimer);
		dslSearchTimer = setTimeout(() => {
			dslCtrl?.abort();
			const ctrl = new AbortController();
			dslCtrl = ctrl;
			queryNotes(q, ctrl.signal)
				.then((r) => { dslResults = r; })
				.catch((e) => { if (!(e instanceof DOMException && e.name === 'AbortError')) dslResults = []; });
		}, 300);
	});

	function pick(item: ResultItem) {
		if (item.kind === 'command') {
			item.cmd.action();
		} else if (item.kind === 'tag') {
			query = `?#${item.tag.tag}`;
		} else {
			onSelect(item.name);
			onClose();
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, allItems.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const item = allItems[selectedIndex];
			if (item) pick(item);
		} else if (e.key === 'Escape') {
			onClose();
		}
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}

	function focusOnMount(el: HTMLElement) {
		el.focus();
	}

	function highlightQuery(text: string, q: string): string {
		const safe = escapeHtml(text);
		const term = q.trim();
		if (!term) return safe;
		const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
		return safe.replace(new RegExp(`(${escaped})`, 'gi'), '<mark>$1</mark>');
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={onBackdropClick}>
	<div class="palette" role="dialog" aria-label="Commands and search" aria-modal="true">
		<!-- Search input -->
		<div class="search-row">
			{#if isCommandMode}
				<span class="mode-icon" aria-hidden="true">⌘</span>
			{:else if isQueryMode}
				<span class="mode-icon query-mode-icon" aria-hidden="true">?</span>
			{:else}
				<svg class="search-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
					<circle cx="8.5" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.5"/>
					<path d="M13.5 13.5L17 17" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
			{/if}
			<input
				bind:value={query}
				onkeydown={onKeydown}
				placeholder={isCommandMode ? 'Type a command…' : isQueryMode ? '#  #tagname  status:done  date:2025…' : 'Search or › for commands'}
				spellcheck="false"
				autocomplete="off"
				use:focusOnMount
			/>
			<kbd class="esc-hint">Esc</kbd>
		</div>

		<!-- Mode hint -->
		{#if !isCommandMode && !isQueryMode && query.trim() === ''}
			<div class="mode-hint">Type <kbd>&gt;</kbd> for commands · <kbd>?</kbd> for queries</div>
		{/if}

		<!-- Results -->
		<ul class="results" role="listbox">
			{#if allItems.length === 0 && query.trim() !== ''}
				<li class="empty">No results</li>
			{:else}
				{#each allItems as item, i (item.kind === 'note' ? 'n:' + item.name : item.kind === 'command' ? 'c:' + item.cmd.id : 't:' + item.tag.tag)}
					<li
						role="option"
						aria-selected={i === selectedIndex}
						class:active={i === selectedIndex}
						class:command-item={item.kind === 'command'}
						class:has-snippet={item.kind === 'note' && !!item.snippet}
						onmouseenter={() => (selectedIndex = i)}
						onmousedown={(e) => { e.preventDefault(); pick(item); }}
					>
						{#if item.kind === 'command'}
							<span class="item-icon cmd-icon" aria-hidden="true">{item.cmd.icon}</span>
							<span class="item-label">{item.cmd.label}</span>
							{#if item.cmd.hint}
								<span class="item-hint">{item.cmd.hint}</span>
							{/if}
						{:else if item.kind === 'tag'}
							<span class="item-icon cmd-icon" aria-hidden="true">#</span>
							<span class="item-label">{item.tag.tag}</span>
							<span class="item-hint">{item.tag.count} note{item.tag.count > 1 ? 's' : ''}</span>
						{:else if item.snippet}
							<div class="snippet-result">
								<div class="snippet-title-row">
									<span class="item-icon note-icon">
										<NoteIcon isIndex={item.meta?.note_type === 'index' || notes.find(n => n.name === item.name)?.is_index} />
									</span>
									<span class="item-label">{item.name}</span>
								</div>
								<!-- eslint-disable-next-line svelte/no-at-html-tags -->
								<p class="item-snippet">{@html highlightQuery(item.snippet, query)}</p>
							</div>
						{:else}
							<span class="item-icon note-icon">
								<NoteIcon isIndex={item.meta?.note_type === 'index' || notes.find(n => n.name === item.name)?.is_index} />
							</span>
							<span class="item-label">{item.name}</span>
							{#if item.meta}
								<span class="item-meta">
									{#if item.meta.status}<span class="meta-chip">{item.meta.status}</span>{/if}
									{#each (item.meta.tags ?? []).slice(0, 3) as tag}<span class="meta-chip tag-chip">{tag}</span>{/each}
									{#if item.meta.date}<span class="meta-chip">{item.meta.date}</span>{/if}
								</span>
							{/if}
						{/if}
					</li>
				{/each}
			{/if}
		</ul>

		{#if allItems.length > 0}
			<div class="footer">
				<span><kbd>↑↓</kbd> navigate</span>
				<span><kbd>↵</kbd> select</span>
				<span><kbd>Esc</kbd> close</span>
			</div>
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.35);
		backdrop-filter: blur(2px);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 12vh;
		z-index: 200;
	}

	.palette {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 12px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
		width: 560px;
		max-width: calc(100vw - 2rem);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		max-height: 70vh;
	}

	/* ── Mobile: bottom sheet ─────────────────────────────────── */
	@media (max-width: 640px) {
		.backdrop {
			align-items: flex-end;
			padding-top: 0;
		}
		.palette {
			width: 100%;
			max-width: 100%;
			border-radius: 16px 16px 0 0;
			max-height: 80vh;
			padding-bottom: env(safe-area-inset-bottom, 0);
		}
	}

	/* ── Layout ──────────────────────────────────────────────── */
	.search-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.search-icon {
		width: 18px;
		height: 18px;
		color: var(--muted);
		flex-shrink: 0;
	}

	.mode-icon {
		font-size: 1rem;
		color: var(--accent);
		flex-shrink: 0;
		width: 18px;
		text-align: center;
	}

	.query-mode-icon {
		font-weight: 700;
	}

	input {
		flex: 1;
		border: none;
		outline: none;
		background: transparent;
		font-size: 1rem;
		font-family: inherit;
		color: var(--text);
		line-height: 1.5;
		min-width: 0;
	}

	input::placeholder {
		color: var(--muted);
	}

	.esc-hint {
		font-size: 0.7rem;
		padding: 0.15rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		color: var(--muted);
		background: var(--sidebar-bg);
		font-family: inherit;
		flex-shrink: 0;
	}

	.mode-hint {
		padding: 0.4rem 1rem;
		font-size: 0.8rem;
		color: var(--muted);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.mode-hint kbd {
		font-size: 0.75rem;
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--sidebar-bg);
		font-family: inherit;
		color: var(--text);
	}

	.results {
		list-style: none;
		margin: 0;
		padding: 0.4rem;
		overflow-y: auto;
		flex: 1;
	}

	.results li {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		padding: 0.5rem 0.65rem;
		border-radius: 7px;
		cursor: pointer;
		font-size: 0.95rem;
		color: var(--text);
		transition: background 80ms;
	}

	.results li.active {
		background: var(--border);
	}

	.results .empty {
		color: var(--muted);
		font-size: 0.9rem;
		cursor: default;
		justify-content: center;
		padding: 1rem;
	}

	.item-icon {
		flex-shrink: 0;
	}

	.note-icon {
		width: 16px;
		height: 16px;
		color: var(--muted);
		display: flex;
		align-items: center;
		position: relative;
		top: 2px;
	}

	.note-icon :global(svg) {
		width: 100%;
		height: 100%;
	}

	.cmd-icon {
		font-size: 0.85rem;
		color: var(--accent);
		width: 16px;
		text-align: center;
	}

	.item-label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.item-hint {
		font-size: 0.78rem;
		color: var(--muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 180px;
	}

	.has-snippet {
		align-items: stretch;
	}

	.snippet-result {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		width: 100%;
		min-width: 0;
	}

	.snippet-title-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.item-snippet {
		font-size: 0.78rem;
		color: var(--muted);
		line-height: 1.45;
		margin: 0;
		padding-left: calc(16px + 0.5rem); /* align with text after icon */
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.item-snippet :global(mark) {
		background: none;
		color: var(--text);
		font-weight: 600;
	}

	.item-meta {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		flex-shrink: 0;
		flex-wrap: nowrap;
		overflow: hidden;
	}

	.meta-chip {
		font-size: 0.7rem;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		color: var(--muted);
		white-space: nowrap;
	}

	.tag-chip {
		color: var(--accent);
		border-color: var(--accent);
		opacity: 0.8;
	}

	.command-item .item-label {
		font-weight: 500;
	}

	.footer {
		display: flex;
		gap: 1rem;
		padding: 0.5rem 1rem;
		border-top: 1px solid var(--border);
		font-size: 0.75rem;
		color: var(--muted);
		background: var(--sidebar-bg);
		flex-shrink: 0;
	}

	kbd {
		font-size: 0.7rem;
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg);
		font-family: inherit;
		color: var(--text);
	}
</style>
