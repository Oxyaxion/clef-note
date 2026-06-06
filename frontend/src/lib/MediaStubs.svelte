<script lang="ts">
	import { deleteNote, type NoteStub } from './api';
	import { emit } from './events';

	interface Props {
		stubs: NoteStub[];
		maxBytes: number;
		onMaxBytesChange: (v: number) => void;
		onNavigate: (name: string) => void;
		onDeleted: (name: string) => void;
	}

	let { stubs, maxBytes, onMaxBytesChange, onNavigate, onDeleted }: Props = $props();

	let selected = $state(new Set<string>());
	let confirming = $state(false);
	let deleting = $state(false);

	const allSelected = $derived(stubs.length > 0 && selected.size === stubs.length);
	const someSelected = $derived(selected.size > 0 && !allSelected);

	const THRESHOLDS = [
		{ label: 'Empty',   value: 0 },
		{ label: '< 100 B', value: 100 },
		{ label: '< 500 B', value: 500 },
		{ label: '< 1 KB',  value: 1000 },
	];

	function formatSize(bytes: number): string {
		if (bytes === 0) return 'empty';
		if (bytes < 1000) return `${bytes} B`;
		return `${(bytes / 1000).toFixed(1)} KB`;
	}

	function displayName(stub: NoteStub): string {
		return stub.title || stub.name.split('/').pop() || stub.name;
	}

	function toggleAll() {
		if (allSelected) {
			selected = new Set();
		} else {
			selected = new Set(stubs.map(s => s.name));
		}
	}

	function toggle(name: string) {
		const next = new Set(selected);
		if (next.has(name)) next.delete(name); else next.add(name);
		selected = next;
	}

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
</script>

<div class="stubs-toolbar">
	<span class="stubs-label">Show notes smaller than:</span>
	<div class="threshold-pills">
		{#each THRESHOLDS as t}
			<button
				class="pill"
				class:active={maxBytes === t.value}
				onclick={() => { onMaxBytesChange(t.value); selected = new Set(); }}
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
					onchange={() => toggle(stub.name)}
					aria-label="Select {stub.name}"
				/>
				<button class="stub-name" onclick={() => onNavigate(stub.name)}>
					{displayName(stub)}
				</button>
				{#if stub.name !== displayName(stub)}
					<span class="stub-path">{stub.name}</span>
				{/if}
				<span class="stub-size">{formatSize(stub.body_len)}</span>
			</li>
		{/each}
	</ul>
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
		color: var(--text-muted);
		white-space: nowrap;
	}

	.threshold-pills { display: flex; gap: 0.35rem; }

	.pill {
		padding: 0.2rem 0.65rem;
		border-radius: 999px;
		font-size: 0.75rem;
		background: var(--bg-alt);
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
		color: var(--text-muted);
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
		background: var(--bg-alt);
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
		background: var(--bg-alt);
		font-size: 0.75rem;
		color: var(--text-muted);
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

	.stub-item.is-selected { background: var(--bg-alt); }

	.stub-check { flex-shrink: 0; cursor: pointer; }

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
		color: var(--text-muted);
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.stub-size {
		font-size: 0.75rem;
		color: var(--text-muted);
		white-space: nowrap;
		margin-left: auto;
	}

	.empty-state {
		padding: 2rem;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.875rem;
	}
</style>
