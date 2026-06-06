<script lang="ts">
	import { untrack } from 'svelte';
	import Editor from '$lib/Editor.svelte';
	import RawSource from '$lib/RawSource.svelte';
	import Backlinks from '$lib/Backlinks.svelte';
	import FrontmatterEditor from '$lib/FrontmatterEditor.svelte';
	import TableOfContents, { type Heading } from '$lib/TableOfContents.svelte';
	import TitleBar from '$lib/TitleBar.svelte';
	import { renameNote, serializeFrontmatter, type Frontmatter } from '$lib/api';
	import { emit } from '$lib/events';

	interface Props {
		selected: string;
		noteContent: string;
		noteFrontmatter: Frontmatter;
		noteNames: string[];
		saving: boolean;
		saveFailed: boolean;
		isMobile: boolean;
		/** Effective lock state from parent (frontmatter lock OR mobile read-only). */
		lockedOverride?: boolean;
		renaming?: boolean;
		focusMode?: boolean;
		rawView?: boolean;
		/** Key that forces the editor/frontmatter to re-sync (bumps after a raw edit). */
		reloadKey: string;
		onEdit: (markdown: string) => void;
		onRawEdit: (raw: string) => void;
		onToggleRaw: () => void;
		onFrontmatterChange: (fm: Frontmatter) => void;
		onNavigate: (name: string) => void;
		onRenamed: (oldName: string, newName: string) => void;
		onOpenPalette: () => void;
	}

	let {
		selected,
		noteContent,
		noteFrontmatter,
		noteNames,
		saving,
		saveFailed,
		isMobile,
		lockedOverride = undefined,
		renaming = $bindable(false),
		focusMode = $bindable(false),
		rawView = false,
		reloadKey,
		onEdit,
		onRawEdit,
		onToggleRaw,
		onFrontmatterChange,
		onNavigate,
		onRenamed,
		onOpenPalette,
	}: Props = $props();

	// Full markdown source (frontmatter block + body) for the raw view.
	// Computed only while the raw view is open: otherwise this would re-allocate
	// the entire note string on every keystroke for nothing.
	const fullMarkdown = $derived(rawView ? serializeFrontmatter(noteFrontmatter) + noteContent : '');

	let renameValue = $state('');
	let renameError = $state('');

	const isIndex = $derived(noteFrontmatter.type === 'index');

	// Use parent-provided effective lock (covers both frontmatter + mobile read-only),
	// or fall back to frontmatter alone when no override is given.
	const isLocked = $derived(lockedOverride !== undefined ? lockedOverride : noteFrontmatter.locked === true);

	function extractHeadings(md: string): Heading[] {
		return Array.from(md.matchAll(/^(#{1,6})\s+(.+?)(?:\s+#+\s*)?$/gm)).map((m) => ({
			level: m[1].length,
			text: m[2].trim(),
		}));
	}

	// The heading scan is O(content). Run it immediately on note switch (so the
	// TOC is correct right away) but debounce it while typing — otherwise every
	// keystroke re-scans the whole note.
	let headings = $state<Heading[]>([]);

	$effect(() => {
		reloadKey; // recompute now on note switch
		headings = extractHeadings(untrack(() => noteContent));
	});

	$effect(() => {
		const md = noteContent; // recompute (debounced) on edits
		const id = setTimeout(() => { headings = extractHeadings(md); }, 250);
		return () => clearTimeout(id);
	});

	const showToc = $derived(headings.length >= 2 && noteFrontmatter.toc !== false);

	// Initialize rename input when rename mode starts
	$effect(() => {
		if (renaming) {
			renameValue = untrack(() => selected);
			renameError = '';
		}
	});

	function disableToc() {
		onFrontmatterChange({ ...noteFrontmatter, toc: false });
	}

	function toggleLock() {
		const fm = { ...noteFrontmatter };
		if (fm.locked) { delete fm.locked; } else { fm.locked = true; }
		onFrontmatterChange(fm);
	}

	async function confirmRename() {
		if (!renaming) return;
		renaming = false;
		const newName = renameValue.trim();
		if (!newName || newName === selected) return;
		const oldName = selected;
		try {
			await renameNote(oldName, newName);
			emit(document, 'notes:changed');
			onRenamed(oldName, newName);
		} catch (e: unknown) {
			renameError = e instanceof Error ? e.message : 'Rename failed';
			renaming = true;
		}
	}

	function cancelRename() {
		renaming = false;
		renameError = '';
	}

</script>

<TitleBar
	{selected}
	{saving}
	{saveFailed}
	{isLocked}
	{focusMode}
	{rawView}
	{isMobile}
	bind:renaming
	bind:renameValue
	renameError={renameError}
	onStartRename={() => (renaming = true)}
	onConfirmRename={confirmRename}
	onCancelRename={cancelRename}
	onToggleLock={toggleLock}
	onToggleFocus={() => (focusMode = !focusMode)}
	onToggleRaw={onToggleRaw}
	onOpenPalette={onOpenPalette}
/>

{#if rawView}
	{#key reloadKey}
		<RawSource value={fullMarkdown} {isLocked} onInput={onRawEdit} />
	{/key}
{:else}
	{#if showToc && !focusMode}
		<TableOfContents {headings} onDisable={disableToc} />
	{/if}

	<div class="editor-area" class:focus-mode={focusMode}>
		{#key reloadKey}
			<FrontmatterEditor
				frontmatter={noteFrontmatter}
				onChange={onFrontmatterChange}
			/>
		{/key}
		<Editor {noteContent} noteKey={reloadKey} {noteNames} {onEdit} {isIndex} {isLocked} />
	</div>

	{#if !focusMode}
		<Backlinks note={selected} onNavigate={onNavigate} />
	{/if}
{/if}

<style>
	/* ── Focus mode ─────────────────────────────────────────── */
	.editor-area {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
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
</style>
