<script lang="ts">
	import { emit } from './events';
	import type { NoteStub } from './api';

	interface Props {
		stubs: NoteStub[];
		maxBytes: number;
		onMaxBytesChange: (v: number) => void;
		onNavigate: (name: string) => void;
	}

	let { stubs, maxBytes, onMaxBytesChange, onNavigate }: Props = $props();

	const THRESHOLDS = [
		{ label: 'Empty',    value: 0 },
		{ label: '< 100 B',  value: 100 },
		{ label: '< 500 B',  value: 500 },
		{ label: '< 1 KB',   value: 1000 },
	];

	function formatSize(bytes: number): string {
		if (bytes === 0) return 'empty';
		if (bytes < 1000) return `${bytes} B`;
		return `${(bytes / 1000).toFixed(1)} KB`;
	}

	function displayName(stub: NoteStub): string {
		return stub.title || stub.name.split('/').pop() || stub.name;
	}
</script>

<div class="stubs-toolbar">
	<span class="stubs-label">Show notes smaller than:</span>
	<div class="threshold-pills">
		{#each THRESHOLDS as t}
			<button
				class="pill"
				class:active={maxBytes === t.value}
				onclick={() => onMaxBytesChange(t.value)}
			>{t.label}</button>
		{/each}
	</div>
</div>

{#if stubs.length === 0}
	<div class="empty-state">No notes found for this threshold.</div>
{:else}
	<ul class="stubs-list">
		{#each stubs as stub}
			<li class="stub-item">
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
	}

	.stubs-label {
		font-size: 0.8rem;
		color: var(--text-muted);
		white-space: nowrap;
	}

	.threshold-pills {
		display: flex;
		gap: 0.35rem;
	}

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

	.stubs-list {
		list-style: none;
		margin: 0;
		padding: 0.5rem 0;
		overflow-y: auto;
		flex: 1;
	}

	.stub-item {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		padding: 0.4rem 1rem;
		border-bottom: 1px solid var(--border);
	}

	.stub-item:last-child {
		border-bottom: none;
	}

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

	.stub-name:hover {
		text-decoration: underline;
	}

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
