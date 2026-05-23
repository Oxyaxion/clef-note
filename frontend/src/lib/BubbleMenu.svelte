<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import { emit } from './events';

	let { editor }: { editor: Editor } = $props();

	let open = $state(false);
	let x = $state(0);
	let y = $state(0);
	let isBold = $state(false);
	let isItalic = $state(false);
	let isStrike = $state(false);
	let isCode = $state(false);
	let isLink = $state(false);

	$effect(() => {
		if (!editor) return;

		function onSelection() {
			const { empty } = editor.state.selection;
			if (empty || !editor.isEditable || editor.isActive('codeBlock')) {
				open = false;
				return;
			}
			const sel = window.getSelection();
			if (!sel || sel.rangeCount === 0) { open = false; return; }
			const rect = sel.getRangeAt(0).getBoundingClientRect();
			if (!rect.width && !rect.height) { open = false; return; }

			x = rect.left + rect.width / 2;
			y = rect.top;
			isBold   = editor.isActive('bold');
			isItalic = editor.isActive('italic');
			isStrike = editor.isActive('strike');
			isCode   = editor.isActive('code');
			isLink   = editor.isActive('link');
			open = true;
		}

		function onBlur() { setTimeout(() => { open = false; }, 0); }

		editor.on('selectionUpdate', onSelection);
		editor.on('blur', onBlur);
		return () => {
			editor.off('selectionUpdate', onSelection);
			editor.off('blur', onBlur);
		};
	});

	function prevent(e: MouseEvent, fn: () => void) {
		e.preventDefault();
		fn();
	}

	function openLinkPrompt() {
		const { from, to, empty } = editor.state.selection;
		const coords = editor.view.coordsAtPos(from);
		emit(editor.view.dom, 'link-prompt', {
			x: coords.left,
			y: coords.bottom + 8,
			currentUrl: editor.getAttributes('link').href ?? '',
			selectedText: empty ? '' : editor.state.doc.textBetween(from, to),
		}, { bubbles: true });
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="bubble" style="left:{x}px;top:{y}px;" role="toolbar" aria-label="Formatting">
		<button
			class:on={isBold}
			onmousedown={(e) => prevent(e, () => editor.chain().focus().toggleBold().run())}
			title="Bold"
		><b>B</b></button>
		<button
			class:on={isItalic}
			onmousedown={(e) => prevent(e, () => editor.chain().focus().toggleItalic().run())}
			title="Italic"
		><i>I</i></button>
		<button
			class:on={isStrike}
			onmousedown={(e) => prevent(e, () => editor.chain().focus().toggleStrike().run())}
			title="Strikethrough"
		><s>S</s></button>
		<span class="sep"></span>
		<button
			class:on={isCode}
			onmousedown={(e) => prevent(e, () => editor.chain().focus().toggleCode().run())}
			title="Inline code"
		><code>`</code></button>
		<span class="sep"></span>
		<button
			class:on={isLink}
			onmousedown={(e) => prevent(e, openLinkPrompt)}
			title="Link"
		>
			<svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
				<path d="M6.5 9.5a4.5 4.5 0 0 0 6.364 0l1.414-1.414a4.5 4.5 0 0 0-6.364-6.364L6.5 3.136"/>
				<path d="M9.5 6.5a4.5 4.5 0 0 0-6.364 0L1.722 7.914a4.5 4.5 0 0 0 6.364 6.364L9.5 12.864"/>
			</svg>
		</button>
	</div>
{/if}

<style>
	.bubble {
		position: fixed;
		z-index: 200;
		transform: translate(-50%, calc(-100% - 8px));
		display: flex;
		align-items: center;
		gap: 1px;
		padding: 3px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 7px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
		pointer-events: all;
	}

	button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 26px;
		border: none;
		background: none;
		color: var(--text);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		font-family: inherit;
		flex-shrink: 0;
	}

	button:hover { background: var(--sidebar-bg); }

	button.on {
		background: color-mix(in srgb, var(--accent) 15%, transparent);
		color: var(--accent);
	}

	button[title="Bold"]            { font-weight: 700; }
	button[title="Italic"]          { font-style: italic; }
	button[title="Strikethrough"]   { text-decoration: line-through; }
	button[title="Inline code"] code { font-family: monospace; font-size: 0.9em; }

	.sep {
		width: 1px;
		height: 16px;
		background: var(--border);
		flex-shrink: 0;
		margin: 0 1px;
	}
</style>
