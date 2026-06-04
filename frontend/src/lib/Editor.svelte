<script lang="ts">
	import { onMount, onDestroy, untrack } from 'svelte';
	import { Editor, InputRule } from '@tiptap/core';
	import { EditorState } from '@tiptap/pm/state';
	import StarterKit from '@tiptap/starter-kit';
	import Placeholder from '@tiptap/extension-placeholder';
	import { Table, TableRow, TableCell, TableHeader } from '@tiptap/extension-table';
	import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
	import { createLowlight, common } from 'lowlight';
	import Typography from '@tiptap/extension-typography';
	import Link from '@tiptap/extension-link';
	import { Markdown, type MarkdownStorage } from 'tiptap-markdown';
	import { TaskListMd, TaskItemMd } from './taskExtensions';
	import { WikiLink } from './wikiLink';
	import { createWikiLinkSuggestion } from './wikiLinkSuggestion';
	import { getAliases, uploadAsset } from './api';
	import { SlashCommand } from './slashCommands';
	import { EmojiShortcodes } from './emojiShortcodes';
	import { QueryBlock } from './queryBlock';
	import { DrawingBlock } from './drawingBlock';
	import { makeCodeBlockNodeView } from './codeBlockNodeView';
	import { ResizableImage } from './resizableImage';
	import { ExitBlockquote, ExitEmptyListItem } from './editorExtensions';
	import { ParagraphMd } from './paragraphMd';
	import BubbleMenu from './BubbleMenu.svelte';
	import LinkPrompt from './LinkPrompt.svelte';
	import TableToolbar from './TableToolbar.svelte';
	import { emit, on } from './events';
	import 'tippy.js/dist/tippy.css';

	const lowlight = createLowlight(common);

	interface Props {
		noteContent: string;
		/** Changes only on note switch — used as the trigger to reload content, not on every keystroke. */
		noteKey: string | null;
		noteNames: string[];
		isIndex?: boolean;
		isLocked?: boolean;
		onEdit: (markdown: string) => void;
	}

	let { noteContent, noteKey, noteNames, isIndex = false, isLocked = false, onEdit }: Props = $props();

	let element: HTMLDivElement;
	let editor: Editor | null = null;  // must NOT be $state — TipTap mutates it internally
	let editorReady = $state(false);   // reactive flag for template
	let isUpdatingFromProp = false;
	let aliasMap: Record<string, string> = {}; // plain let — read lazily in closure, not in template

	// DOM-level handlers stored here so onDestroy can remove them
	let _imgPasteHandler: ((e: ClipboardEvent) => void) | null = null;
	let _imgDropHandler: ((e: DragEvent) => void) | null = null;
	let _offInsertImage: (() => void) | null = null;

	function getMarkdown(ed: Editor): string {
		return (ed.storage as unknown as { markdown: MarkdownStorage }).markdown.getMarkdown();
	}

	// Load aliases on mount; reload only on rename/delete (notes:changed), not on every keystroke.
	// noteNames changes on every save, so watching it would fire a GET /api/aliases per keystroke.
	$effect(() => {
		getAliases().then(m => { aliasMap = m; }).catch(() => {});
		return on(document, 'notes:changed', () => {
			getAliases().then(m => { aliasMap = m; }).catch(() => {});
		});
	});

	// noteNames is a Svelte 5 prop getter — the closure reads the current value lazily on each call
	onMount(() => {
		const WikiLinkSuggestion = createWikiLinkSuggestion(() => noteNames, () => aliasMap);

		editor = new Editor({
			element,
			extensions: [
				StarterKit.configure({ codeBlock: false, paragraph: false }),
				ParagraphMd,
				Placeholder.configure({ placeholder: 'Start writing… (type / for commands)' }),
				Markdown.configure({ html: false, transformPastedText: true }),
				ResizableImage,
				TaskListMd,
				TaskItemMd,
				CodeBlockLowlight.configure({ lowlight }).extend({ addNodeView: makeCodeBlockNodeView }),
				Table.configure({ resizable: false }),
				TableRow,
				TableCell,
				TableHeader,
				Typography,
				Link.configure({
					openOnClick: false,
					autolink: true,
					linkOnPaste: true,
				}).extend({
					// autolink:true sets inclusive:true, which causes typing after a link
					// to continue adding link marks. Override to always be non-inclusive.
					inclusive() { return false; },
					addInputRules() {
						return [
							new InputRule({
								find: /\[([^\]]+)\]\(([^)\s]+)\)$/,
								handler: ({ chain, range, match }) => {
									const label = match[1];
									const url = match[2];
									const href = /^https?:\/\//.test(url) ? url : `https://${url}`;
									chain()
										.deleteRange(range)
										.insertContent({ type: 'text', text: label, marks: [{ type: 'link', attrs: { href } }] })
										.run();
								},
							}),
						];
					},
					addKeyboardShortcuts() {
						return {
							'Mod-Shift-k': () => {
								const ed = this.editor;
								const { from, to, empty } = ed.state.selection;
								const coords = ed.view.coordsAtPos(from);
								emit(ed.view.dom, 'link-prompt', {
									x: coords.left,
									y: coords.bottom + 8,
									currentUrl: ed.getAttributes('link').href ?? '',
									selectedText: empty ? '' : ed.state.doc.textBetween(from, to),
								}, { bubbles: true });
								return true;
							},
						};
					},
				}),
				WikiLink,
				WikiLinkSuggestion,
				SlashCommand,
				EmojiShortcodes,
				QueryBlock,
				DrawingBlock,
				ExitBlockquote,
				ExitEmptyListItem,
			],
			content: noteContent,
			editorProps: {
				attributes: {
					class: 'tiptap-editor',
					spellcheck: 'true',
				},
			},
			onUpdate({ editor: ed }) {
				if (isUpdatingFromProp) return;
				onEdit(getMarkdown(ed));
			},
		});

		_offInsertImage = on(document, 'insert-image', (url) => {
			editor?.chain().focus().setImage({ src: url }).run();
		});

		// Capture-phase listeners run before ProseMirror sees the event.
		// Return early (without stopImmediatePropagation) for non-image pastes so
		// normal text/markdown paste continues to work.
		_imgPasteHandler = (e: ClipboardEvent) => {
			const imgItem = Array.from(e.clipboardData?.items ?? [])
				.find(it => it.type.startsWith('image/'));
			if (!imgItem) return;
			const file = imgItem.getAsFile();
			if (!file) return;
			e.preventDefault();
			e.stopImmediatePropagation();
			const ext = file.type.split('/')[1]?.replace('jpeg', 'jpg') ?? 'png';
			const named = new File([file], `paste-${Date.now()}.${ext}`, { type: file.type });
			uploadAsset(named)
				.then(url => { editor?.chain().focus().setImage({ src: url }).run(); })
				.catch(err => console.error('Image upload failed', err));
		};
		_imgDropHandler = (e: DragEvent) => {
			const imgFile = Array.from(e.dataTransfer?.files ?? [])
				.find(f => f.type.startsWith('image/'));
			if (!imgFile) return;
			e.preventDefault();
			e.stopImmediatePropagation();
			uploadAsset(imgFile)
				.then(url => { editor?.chain().focus().setImage({ src: url }).run(); })
				.catch(err => console.error('Image upload failed', err));
		};
		element.addEventListener('paste', _imgPasteHandler as EventListener, true);
		element.addEventListener('drop', _imgDropHandler as EventListener, true);

		editorReady = true;
	});

	// Runs only when noteKey changes (note switch), not on every keystroke.
	// noteContent is read untracked so typing never re-triggers this effect.
	$effect(() => {
		noteKey; // reactive dependency — changes only on note switch
		if (!editor) return;
		const content = untrack(() => noteContent);
		isUpdatingFromProp = true;
		editor.commands.setContent(content, { emitUpdate: false });
		// Replace state entirely so undo history from the previous note is cleared.
		// Without this, Ctrl+Z after switching notes could revert to another note's content.
		const { schema, plugins, doc } = editor.state;
		editor.view.updateState(EditorState.create({ schema, plugins, doc }));
		element.scrollTop = 0;
		isUpdatingFromProp = false;
	});

	$effect(() => {
		if (!editorReady || !editor) return;
		editor.setEditable(!isLocked);
	});

	onDestroy(() => {
		_offInsertImage?.();
		if (_imgPasteHandler) element.removeEventListener('paste', _imgPasteHandler as EventListener, true);
		if (_imgDropHandler) element.removeEventListener('drop', _imgDropHandler as EventListener, true);
		editor?.destroy();
		editor = null;
		editorReady = false;
	});
</script>

<div bind:this={element} class="editor-wrap" class:index-page={isIndex} class:locked={isLocked}></div>

{#if editorReady && editor}
	<BubbleMenu {editor} />
	<LinkPrompt {editor} />
	<TableToolbar {editor} />
{/if}

<style>
	.editor-wrap {
		flex: 1;
		overflow-y: auto;
		padding: 2rem 3rem;
	}

	.locked :global(.tiptap-editor) {
		cursor: default;
		user-select: text;
	}

	/* ── Index page layout ──────────────────────────────── */
	.index-page :global(.tiptap-editor h1) {
		text-align: center;
		font-size: 2.4rem;
		margin-bottom: 2rem;
		padding-bottom: 0.8rem;
		border-bottom: 2px solid var(--border);
	}

	.index-page :global(.tiptap-editor) {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 380px), 1fr));
		gap: 1rem;
		align-items: start;
	}

	/* Everything except query blocks spans full width */
	.index-page :global(.tiptap-editor > *:not(.query-block)) {
		grid-column: 1 / -1;
	}

	/* Query blocks in the grid */
	.index-page :global(.query-block) {
		margin: 0;
	}

	:global(.tiptap-editor) {
		outline: none;
		min-height: 100%;
		font-family: 'Inter', system-ui, sans-serif;
		font-size: 1.1rem;
		line-height: 1.7;
		color: var(--text);
	}

	/* Headings */
	:global(.tiptap-editor h1) { font-size: 2rem; font-weight: 600; margin: 1.5rem 0 0.8rem; line-height: 1.3; }
	:global(.tiptap-editor h2) { font-size: 1.5rem; font-weight: 600; margin: 1.2rem 0 0.65rem; line-height: 1.3; }
	:global(.tiptap-editor h3) { font-size: 1.2rem; font-weight: 600; margin: 1rem 0 0.5rem; line-height: 1.3; }
	:global(.tiptap-editor h4) { font-size: 1.05rem; font-weight: 600; margin: 0.9rem 0 0.45rem; line-height: 1.3; }
	:global(.tiptap-editor h5) { font-size: 0.95rem; font-weight: 600; margin: 0.8rem 0 0.4rem; line-height: 1.3; }
	:global(.tiptap-editor h6) { font-size: 0.88rem; font-weight: 600; margin: 0.8rem 0 0.4rem; line-height: 1.3; }

	/* Paragraphs */
	:global(.tiptap-editor p) { margin: 0 0 0.6rem; }

	/* Links */
	:global(.tiptap-editor a) {
		color: var(--accent);
		text-decoration: underline;
		text-underline-offset: 2px;
		cursor: pointer;
	}
	:global(.tiptap-editor a:hover) { opacity: 0.8; }

	:global(.tiptap-editor p.is-editor-empty:first-child::before) {
		content: attr(data-placeholder);
		float: left;
		color: var(--muted);
		pointer-events: none;
		height: 0;
	}

	/* Lists */
	:global(.tiptap-editor ul) { list-style-type: disc; padding-left: 1.5rem; margin: 0.4rem 0 0.8rem; }
	:global(.tiptap-editor ol) { list-style-type: decimal; padding-left: 1.5rem; margin: 0.4rem 0 0.8rem; }
	:global(.tiptap-editor ul ul) { list-style-type: circle; }
	:global(.tiptap-editor ul ul ul) { list-style-type: square; }
	:global(.tiptap-editor li) { margin: 0.2rem 0; }

	/* Code inline */
	:global(.tiptap-editor code) {
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 0.88em;
		background: var(--border);
		padding: 0.15em 0.4em;
		border-radius: 3px;
	}

	/* Code block wrapper */
	:global(.tiptap-editor .code-block-wrap) {
		border-left: 3px solid var(--border);
		border-radius: 0 4px 4px 0;
		margin: 0.8rem 0;
		background: var(--code-bg, color-mix(in srgb, var(--sidebar-bg) 70%, transparent));
		position: relative;
	}

	/* Header spans full width: lang left, copy right */
	:global(.tiptap-editor .code-block-header) {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.3rem 0.5rem;
		pointer-events: none;
		z-index: 2;
	}

	:global(.tiptap-editor .code-lang) {
		pointer-events: auto;
		font-size: 0.67rem;
		font-family: 'JetBrains Mono', monospace;
		color: var(--muted);
		text-transform: lowercase;
		letter-spacing: 0.03em;
		cursor: pointer;
		padding: 0.1rem 0.35rem;
		border-radius: 3px;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		transition: background 80ms, color 80ms;
	}
	:global(.tiptap-editor .code-lang:hover) {
		background: var(--border);
		color: var(--text);
	}

	:global(.tiptap-editor .code-lang-input) {
		pointer-events: auto;
		font-size: 0.67rem;
		font-family: 'JetBrains Mono', monospace;
		color: var(--text);
		background: var(--bg);
		border: 1px solid var(--accent);
		border-radius: 3px;
		outline: none;
		padding: 0.1rem 0.3rem;
		width: 7rem;
	}

	/* Copy: hidden by default, revealed on hover */
	:global(.tiptap-editor .code-copy-btn) {
		font-size: 0.65rem;
		font-family: inherit;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.1rem 0.4rem;
		color: var(--muted);
		cursor: pointer;
		opacity: 0;
		pointer-events: none;
		transition: opacity 120ms, background 80ms, color 80ms;
	}
	:global(.tiptap-editor .code-block-wrap:hover .code-copy-btn),
	:global(.tiptap-editor .code-block-wrap:focus-within .code-copy-btn) {
		opacity: 1;
		pointer-events: auto;
	}
	:global(.tiptap-editor .code-copy-btn:hover) {
		background: var(--border);
		color: var(--text);
	}

	/* Extra top padding so first code line clears the header badge */
	:global(.tiptap-editor .code-block-wrap pre) {
		margin: 0;
		padding: 2.1rem 1rem 0.85rem;
		overflow-x: auto;
		background: none;
		border: none;
		border-radius: 0;
	}

	:global(.tiptap-editor .code-block-wrap pre code) {
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
		font-size: 0.88rem;
		line-height: 1.6;
		background: none;
		padding: 0;
		border-radius: 0;
		color: var(--text);
	}

	/* Blockquote */
	:global(.tiptap-editor blockquote) {
		border-left: 4px solid var(--accent);
		background: color-mix(in srgb, var(--accent) 14%, var(--bg));
		color: var(--blockquote-text);
		margin: 0.6rem 0;
		padding: 0.4em 0.9em;
		border-radius: 0 4px 4px 0;
	}
	:global(.tiptap-editor blockquote p) {
		font-style: italic;
		font-weight: 500;
		color: inherit;
	}

	/* Table */
	:global(.tiptap-editor table) { border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 0.95rem; }
	:global(.tiptap-editor th, .tiptap-editor td) { border: 1px solid var(--border); padding: 0.5rem 0.75rem; text-align: left; vertical-align: top; }
	:global(.tiptap-editor th) { background: var(--sidebar-bg); font-weight: 600; }
	:global(.tiptap-editor .selectedCell) { background: color-mix(in srgb, var(--accent) 8%, transparent); }

	/* Images */
	:global(.image-wrapper) {
		display: inline-block;
		position: relative;
		line-height: 0;
		max-width: 100%;
		vertical-align: bottom;
	}
	:global(.image-wrapper img) {
		display: block;
		max-width: 100%;
		height: auto;
		border-radius: 6px;
	}
	:global(.image-resize-handle) {
		position: absolute;
		right: -5px;
		bottom: -5px;
		width: 12px;
		height: 12px;
		background: var(--accent);
		border: 2px solid var(--bg);
		border-radius: 50%;
		cursor: se-resize;
		opacity: 0;
		transition: opacity 120ms;
	}
	:global(.image-wrapper:hover .image-resize-handle) { opacity: 1; }

	/* Horizontal rule */
	:global(.tiptap-editor hr) { border: none; border-top: 1px solid var(--border); margin: 1.5rem 0; }

	/* Slash & wiki-link menus */
	:global(.slash-menu) {
		background: var(--bg, #fff);
		border: 1px solid var(--border, #e5e5e5);
		border-radius: 10px;
		box-shadow: 0 8px 30px rgba(0,0,0,0.14);
		padding: 6px;
		min-width: 240px;
		max-height: 340px;
		overflow-y: auto;
	}
	:global(.slash-menu-item) {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.45rem 0.6rem;
		cursor: pointer;
		border-radius: 6px;
		color: var(--text, #1a1a1a);
	}
	:global(.slash-menu-item.selected), :global(.slash-menu-item:hover) { background: var(--border, #e5e5e5); }
	:global(.slash-menu-icon) {
		width: 32px; height: 32px;
		display: flex; align-items: center; justify-content: center;
		background: var(--sidebar-bg, #f7f7f7);
		border: 1px solid var(--border, #e5e5e5);
		border-radius: 6px;
		font-size: 0.72rem; font-weight: 700;
		font-family: 'JetBrains Mono', monospace;
		flex-shrink: 0;
		color: var(--text, #1a1a1a);
	}
	:global(.slash-menu-text) { display: flex; flex-direction: column; }
	:global(.slash-menu-title) { font-size: 0.88rem; font-weight: 500; line-height: 1.3; }
	:global(.slash-menu-desc)  { font-size: 0.76rem; color: var(--muted, #6b7280); line-height: 1.3; }
	:global(.slash-menu-empty) { padding: 0.75rem 1rem; font-size: 0.85rem; color: var(--muted, #6b7280); }

	/* Mobile: menu docked above the on-screen keyboard (full width).
	   `bottom` and `max-height` are set inline from the visualViewport. */
	:global(.slash-menu--docked) {
		position: fixed;
		left: 0;
		right: 0;
		width: 100%;
		min-width: 0;
		max-height: 50vh;
		border-radius: 14px 14px 0 0;
		border-left: none;
		border-right: none;
		border-bottom: none;
		box-shadow: 0 -8px 30px rgba(0, 0, 0, 0.18);
		padding-bottom: calc(6px + env(safe-area-inset-bottom, 0));
		z-index: 250;
	}

	@media (max-width: 640px) {
		/* Larger tap targets on the docked menu */
		:global(.slash-menu--docked .slash-menu-item) { padding: 0.65rem 0.7rem; }
	}
</style>
