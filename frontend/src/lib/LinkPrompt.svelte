<script lang="ts">
	import { tick } from 'svelte';
	import type { Editor } from '@tiptap/core';
	import { emit, on } from './events';
	import { isSafeHref } from './utils';

	interface Props {
		editor: Editor;
	}

	let { editor }: Props = $props();

	// ── Tooltip (shown when cursor rests inside a link) ────────────────────────
	let tooltipOpen = $state(false);
	let tooltipX = $state(0);
	let tooltipY = $state(0);
	let tooltipHref = $state('');

	// ── Prompt (edit / insert link) ────────────────────────────────────────────
	let promptOpen = $state(false);
	let promptX = $state(0);
	let promptY = $state(0);
	let promptUrl = $state('');
	let promptText = $state('');
	let hasSelection = $state(false);
	let hasExisting = $state(false);
	let urlInput = $state<HTMLInputElement | null>(null);
	let textInput = $state<HTMLInputElement | null>(null);

	function syncTooltip() {
		const { from, empty } = editor.state.selection;
		if (!empty || !editor.isActive('link')) {
			tooltipOpen = false;
			return;
		}
		// Anchor the tooltip to the full <a> span, not just the cursor position
		const { node } = editor.view.domAtPos(from);
		let el: Node | null = node instanceof Text ? node.parentElement : node;
		while (el && !(el instanceof HTMLAnchorElement)) el = (el as Element).parentElement;
		if (el instanceof HTMLAnchorElement) {
			const rect = el.getBoundingClientRect();
			tooltipX = rect.left;
			tooltipY = rect.bottom + 4;
		} else {
			const coords = editor.view.coordsAtPos(from);
			tooltipX = coords.left;
			tooltipY = coords.bottom + 4;
		}
		tooltipHref = editor.getAttributes('link').href ?? '';
		tooltipOpen = true;
	}

	async function openPrompt(detail: { x: number; y: number; currentUrl: string; selectedText: string }) {
		tooltipOpen = false;
		const { x, y, currentUrl, selectedText } = detail;
		promptUrl = currentUrl ?? '';
		promptText = '';
		hasExisting = !!currentUrl;
		hasSelection = !!selectedText;
		promptX = x;
		promptY = y;
		promptOpen = true;
		await tick();
		urlInput?.focus();
		urlInput?.select();
	}

	function applyLink() {
		promptOpen = false;
		const url = promptUrl.trim();
		if (!url) {
			if (hasExisting) editor.chain().focus().unsetLink().run();
			else editor.chain().focus().run();
			return;
		}
		const href = /^https?:\/\//.test(url) ? url : `https://${url}`;
		if (hasSelection) {
			editor.chain().focus().setLink({ href }).run();
		} else if (hasExisting) {
			editor.chain().focus().extendMarkRange('link').setLink({ href }).run();
		} else {
			const label = promptText.trim() || href;
			editor.chain().focus().insertContent({ type: 'text', text: label, marks: [{ type: 'link', attrs: { href } }] }).run();
		}
	}

	function removeLink() {
		promptOpen = false;
		if (hasSelection) {
			editor.chain().focus().unsetLink().run();
		} else {
			editor.chain().focus().extendMarkRange('link').unsetLink().run();
		}
	}

	function closePrompt() {
		promptOpen = false;
		editor.chain().focus().run();
	}

	function editFromTooltip() {
		tooltipOpen = false;
		const { from } = editor.state.selection;
		const coords = editor.view.coordsAtPos(from);
		emit(editor.view.dom, 'link-prompt',
			{ x: coords.left, y: coords.bottom + 8, currentUrl: tooltipHref, selectedText: '' },
			{ bubbles: true }
		);
	}

	function removeFromTooltip() {
		editor.chain().focus().extendMarkRange('link').unsetLink().run();
		tooltipOpen = false;
	}

	$effect(() => {
		const onSelection = () => syncTooltip();
		// Defer to avoid state_unsafe_mutation during Svelte's synchronous commit phase
		const onBlur = () => { setTimeout(() => { tooltipOpen = false; }, 0); };
		editor.on('selectionUpdate', onSelection);
		editor.on('blur', onBlur);
		return () => {
			editor.off('selectionUpdate', onSelection);
			editor.off('blur', onBlur);
		};
	});

	$effect(() => {
		// link-prompt is dispatched on editor.view.dom with bubbles:true — it reaches document
		return on(document, 'link-prompt', openPrompt);
	});
</script>

{#if tooltipOpen}
	<div class="link-tooltip" role="toolbar" aria-label="Link actions" tabindex="-1" style="left:{tooltipX}px;top:{tooltipY}px;" onmousedown={(e) => e.preventDefault()}>
		<span class="lt-url" title={tooltipHref}>
			{tooltipHref.length > 40 ? tooltipHref.slice(0, 40) + '…' : tooltipHref}
		</span>
		<span class="lt-sep"></span>
		<button onclick={() => { if (isSafeHref(tooltipHref)) window.open(tooltipHref, '_blank', 'noopener noreferrer'); }} title="Open">↗</button>
		<button onmousedown={(e) => { e.preventDefault(); editFromTooltip(); }} title="Edit">Edit</button>
		<button onmousedown={(e) => { e.preventDefault(); removeFromTooltip(); }} title="Remove" class="lt-remove">×</button>
	</div>
{/if}

{#if promptOpen}
	<button class="link-backdrop" onclick={closePrompt} aria-label="Close link prompt" tabindex="-1"></button>
	<div class="link-prompt" style="left:{promptX}px; top:{promptY}px;">
		<input
			bind:this={urlInput}
			bind:value={promptUrl}
			type="url"
			placeholder="https://…"
			autocomplete="off"
			spellcheck={false}
			onkeydown={(e) => {
				if (e.key === 'Enter') { e.preventDefault(); hasSelection ? applyLink() : textInput?.focus(); }
				if (e.key === 'Escape') closePrompt();
			}}
		/>
		{#if !hasSelection}
			<input
				bind:this={textInput}
				bind:value={promptText}
				type="text"
				placeholder="Display text (optional)"
				autocomplete="off"
				spellcheck={false}
				onkeydown={(e) => {
					if (e.key === 'Enter') { e.preventDefault(); applyLink(); }
					if (e.key === 'Escape') closePrompt();
				}}
			/>
		{/if}
		<div class="lp-actions">
			<button class="lp-apply" onmousedown={(e) => { e.preventDefault(); applyLink(); }}>Apply</button>
			{#if hasExisting}
				<button class="lp-remove" onmousedown={(e) => { e.preventDefault(); removeLink(); }}>Remove</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	/* ── Tooltip ──────────────────────────────────────────────── */
	.link-tooltip {
		position: fixed;
		z-index: 200;
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 3px 6px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		box-shadow: 0 3px 12px rgba(0, 0, 0, 0.12);
		font-size: 0.75rem;
		white-space: nowrap;
		max-width: 380px;
	}

	.lt-url {
		color: var(--muted);
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 200px;
		display: inline-block;
	}

	.lt-sep {
		width: 1px;
		height: 14px;
		background: var(--border);
		flex-shrink: 0;
		margin: 0 3px;
	}

	.link-tooltip button {
		border: none;
		background: none;
		color: var(--text);
		cursor: pointer;
		padding: 2px 5px;
		border-radius: 4px;
		font-size: 0.75rem;
		font-family: inherit;
	}

	.link-tooltip button:hover { background: var(--sidebar-bg); }
	.link-tooltip button.lt-remove { color: var(--color-danger); }
	.link-tooltip button.lt-remove:hover { background: color-mix(in srgb, var(--color-danger) 10%, transparent); }

	/* ── Prompt ───────────────────────────────────────────────── */
	.link-backdrop {
		position: fixed;
		inset: 0;
		z-index: 299;
		background: none;
		border: none;
		padding: 0;
		cursor: default;
	}

	.link-prompt {
		position: fixed;
		z-index: 300;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.5rem;
		box-shadow: 0 4px 20px rgba(0,0,0,0.14);
		min-width: 280px;
	}

	.link-prompt input {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.28rem 0.55rem;
		font-size: 0.83rem;
		color: var(--text);
		font-family: inherit;
		outline: none;
		width: 100%;
		box-sizing: border-box;
	}

	.link-prompt input:focus { border-color: var(--accent); }

	.lp-actions {
		display: flex;
		gap: 0.3rem;
	}

	.lp-apply,
	.lp-remove {
		border-radius: 5px;
		padding: 0.22rem 0.7rem;
		font-size: 0.78rem;
		font-family: inherit;
		cursor: pointer;
		white-space: nowrap;
		border: 1px solid var(--border);
	}

	.lp-apply {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}

	.lp-apply:hover { opacity: 0.88; }

	.lp-remove {
		background: none;
		color: var(--color-danger);
	}

	.lp-remove:hover { background: color-mix(in srgb, var(--color-danger) 10%, transparent); }
</style>
