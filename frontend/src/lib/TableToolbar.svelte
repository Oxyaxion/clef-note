<script lang="ts">
	import type { Editor } from '@tiptap/core';

	interface Props {
		editor: Editor;
	}

	let { editor }: Props = $props();

	let active = $state(false);
	let toolbarStyle = $state('');

	function sync() {
		if (!editor.isActive('table')) {
			active = false;
			return;
		}
		// Walk up from the selection anchor to find the <table> DOM node
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		let el: Node | null = sel.getRangeAt(0).commonAncestorContainer;
		if (el.nodeType === Node.TEXT_NODE) el = el.parentElement;
		while (el && (el as Element).tagName !== 'TABLE') el = (el as Element).parentElement;
		if (!el) return;

		const rect = (el as Element).getBoundingClientRect();
		active = true;
		const top = Math.max(4, rect.top - 38);
		toolbarStyle = `top:${top}px;left:${rect.left}px;width:${rect.width}px`;
	}

	$effect(() => {
		const onSelection = () => sync();
		// Defer to avoid state_unsafe_mutation during Svelte's synchronous commit phase
		const onBlur = () => { setTimeout(() => { active = false; }, 0); };
		editor.on('selectionUpdate', onSelection);
		editor.on('blur', onBlur);
		return () => {
			editor.off('selectionUpdate', onSelection);
			editor.off('blur', onBlur);
		};
	});
</script>

{#if active}
	<div class="table-toolbar" role="toolbar" tabindex="-1" aria-label="Table actions"
		style={toolbarStyle} onmousedown={(e) => e.preventDefault()}>
		<span class="tb-group">
			<button onclick={() => editor.chain().focus().addColumnBefore().run()} title="Add column before">
				<svg viewBox="0 0 16 16" fill="none"><rect x="1" y="3" width="4" height="10" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M9 8h6M12 5v6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
				Col ←
			</button>
			<button onclick={() => editor.chain().focus().addColumnAfter().run()} title="Add column after">
				Col →
				<svg viewBox="0 0 16 16" fill="none"><rect x="11" y="3" width="4" height="10" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M1 8h6M4 5v6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
			</button>
		</span>
		<span class="tb-sep"></span>
		<span class="tb-group">
			<button onclick={() => editor.chain().focus().addRowBefore().run()} title="Add row above">
				<svg viewBox="0 0 16 16" fill="none"><rect x="3" y="1" width="10" height="4" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M8 9v6M5 12h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
				Row ↑
			</button>
			<button onclick={() => editor.chain().focus().addRowAfter().run()} title="Add row below">
				Row ↓
				<svg viewBox="0 0 16 16" fill="none"><rect x="3" y="11" width="10" height="4" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M8 1v6M5 4h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
			</button>
		</span>
		<span class="tb-sep"></span>
		<span class="tb-group">
			<button class="danger" onclick={() => editor.chain().focus().deleteColumn().run()} title="Delete column">
				<svg viewBox="0 0 16 16" fill="none"><rect x="3" y="3" width="10" height="10" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M8 6v4M6 8h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" transform="rotate(45 8 8)"/></svg>
				Col
			</button>
			<button class="danger" onclick={() => editor.chain().focus().deleteRow().run()} title="Delete row">
				<svg viewBox="0 0 16 16" fill="none"><rect x="3" y="3" width="10" height="10" rx="1" stroke="currentColor" stroke-width="1.2"/><path d="M8 6v4M6 8h4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" transform="rotate(45 8 8)"/></svg>
				Row
			</button>
			<button class="danger" onclick={() => editor.chain().focus().deleteTable().run()} title="Delete table">
				<svg viewBox="0 0 16 16" fill="none"><path d="M2 4h12M2 8h12M2 12h12M4 2v12M8 2v12M12 2v12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" opacity=".4"/><path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
				Table
			</button>
		</span>
	</div>
{/if}

<style>
	.table-toolbar {
		position: fixed;
		z-index: 50;
		display: flex;
		align-items: center;
		gap: 2px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 3px 5px;
		box-shadow: 0 4px 16px rgba(0,0,0,0.12);
		font-size: 0.72rem;
		white-space: nowrap;
		overflow-x: auto;
	}

	button {
		display: flex;
		align-items: center;
		gap: 3px;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		font-size: 0.72rem;
		font-family: inherit;
		padding: 3px 6px;
		border-radius: 5px;
		transition: background 80ms, color 80ms;
		white-space: nowrap;
	}

	button svg {
		width: 12px;
		height: 12px;
		flex-shrink: 0;
	}

	button:hover {
		background: var(--border);
		color: var(--text);
	}

	button.danger:hover {
		background: color-mix(in srgb, var(--color-danger) 15%, transparent);
		color: var(--color-danger);
	}

	.tb-sep {
		width: 1px;
		height: 16px;
		background: var(--border);
		flex-shrink: 0;
		margin: 0 2px;
	}

	.tb-group {
		display: flex;
		align-items: center;
		gap: 1px;
	}
</style>
