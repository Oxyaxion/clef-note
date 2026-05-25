<script lang="ts">
	import { untrack } from 'svelte';
	import Editor from '$lib/Editor.svelte';
	import Backlinks from '$lib/Backlinks.svelte';
	import FrontmatterEditor from '$lib/FrontmatterEditor.svelte';
	import TableOfContents, { type Heading } from '$lib/TableOfContents.svelte';
	import TitleBar from '$lib/TitleBar.svelte';
	import { renameNote, type Frontmatter } from '$lib/api';
	import { emit } from '$lib/events';

	interface Props {
		selected: string;
		noteContent: string;
		noteFrontmatter: Frontmatter;
		noteNames: string[];
		saving: boolean;
		saveFailed: boolean;
		isMobile: boolean;
		renaming?: boolean;
		focusMode?: boolean;
		onEdit: (markdown: string) => void;
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
		renaming = $bindable(false),
		focusMode = $bindable(false),
		onEdit,
		onFrontmatterChange,
		onNavigate,
		onRenamed,
		onOpenPalette,
	}: Props = $props();

	let renameValue = $state('');
	let renameError = $state('');

	const isIndex = $derived(noteFrontmatter.type === 'index');
	const isLocked = $derived(noteFrontmatter.locked === true);

	const headings = $derived<Heading[]>(
		Array.from(noteContent.matchAll(/^(#{1,6})\s+(.+?)(?:\s+#+\s*)?$/gm)).map((m) => ({
			level: m[1].length,
			text: m[2].trim(),
		}))
	);

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
		if (isLocked) {
			delete fm.locked;
		} else {
			fm.locked = true;
		}
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
	{isMobile}
	bind:renaming
	bind:renameValue
	renameError={renameError}
	onStartRename={() => (renaming = true)}
	onConfirmRename={confirmRename}
	onCancelRename={cancelRename}
	onToggleLock={toggleLock}
	onToggleFocus={() => (focusMode = !focusMode)}
	onOpenPalette={onOpenPalette}
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
	{/key}
	<Editor {noteContent} noteKey={selected} {noteNames} {onEdit} {isIndex} {isLocked} />
</div>

{#if !focusMode}
	<Backlinks note={selected} onNavigate={onNavigate} />
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
