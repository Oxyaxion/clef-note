<script lang="ts">
	interface Props {
		drawings: string[];
		drawingPreviews: Record<string, string>;
		usedDrawings: Set<string> | null;
		onPreview: (name: string) => void;
		onDelete: (name: string) => void;
	}

	let { drawings, drawingPreviews, usedDrawings, onPreview, onDelete }: Props = $props();

	let filter = $state<'all' | 'used' | 'orphaned'>('all');

	const filtered = $derived(
		filter === 'all' || !usedDrawings ? drawings :
		filter === 'used' ? drawings.filter(d => usedDrawings!.has(d)) :
		drawings.filter(d => !usedDrawings!.has(d))
	);
</script>

{#if usedDrawings !== null}
	<div class="toolbar">
		<button class="pill" class:active={filter === 'all'} onclick={() => (filter = 'all')}>All</button>
		<button class="pill" class:active={filter === 'used'} onclick={() => (filter = 'used')}>Used</button>
		<button class="pill pill-orphaned" class:active={filter === 'orphaned'} onclick={() => (filter = 'orphaned')}>Orphaned</button>
	</div>
{/if}

{#if drawings.length === 0}
	<div class="empty-state">No drawings yet. Insert one with <kbd>/drawing</kbd>.</div>
{:else if filtered.length === 0}
	<div class="empty-state">No {filter} drawings.</div>
{:else}
	<div class="grid">
		{#each filtered as name (name)}
			{@const isOrphaned = usedDrawings !== null && !usedDrawings.has(name)}
			<div class="card" class:orphaned={isOrphaned}>
				<button class="card-thumb drawing-thumb" onclick={() => onPreview(name)}>
					{#if drawingPreviews[name]}
						<!-- eslint-disable-next-line svelte/no-at-html-tags -->
						{@html drawingPreviews[name]}
					{:else}
						<span class="thumb-icon">✎</span>
					{/if}
					<div class="thumb-overlay">
						<svg viewBox="0 0 20 20" fill="none"><path d="M8 3H3v5M17 3h-5M3 12v5h5M12 17h5v-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
					</div>
				</button>
				<div class="card-footer">
					<div class="card-info">
						<span class="card-name" title={name}>{name}</span>
					</div>
					{#if isOrphaned}
						<span class="badge-orphaned">orphaned</span>
					{/if}
					<button class="card-delete" title="Delete" onclick={() => onDelete(name)}>✕</button>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.toolbar {
		display: flex;
		gap: 0.35rem;
		padding: 0.6rem 1rem;
		border-bottom: 1px solid var(--border);
		margin-bottom: 1rem;
	}

	.pill {
		padding: 0.2rem 0.65rem;
		border-radius: 999px;
		font-size: 0.75rem;
		font-family: inherit;
		background: none;
		border: 1px solid var(--border);
		color: var(--muted);
		cursor: pointer;
		transition: background 80ms, color 80ms;
	}
	.pill:hover { color: var(--text); background: var(--border); }
	.pill.active { color: var(--text); background: var(--border); }
	.pill-orphaned.active { color: #e09050; background: rgba(224, 144, 80, 0.1); border-color: rgba(224, 144, 80, 0.3); }

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 12rem;
		color: var(--muted);
		font-size: 0.9rem;
		font-style: italic;
	}
	.empty-state kbd {
		font-style: normal;
		font-size: 0.8rem;
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--sidebar-bg);
		font-family: inherit;
		color: var(--text);
		margin: 0 0.1rem;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 1rem;
	}

	.card {
		border: 1px solid var(--border);
		border-radius: 8px;
		overflow: hidden;
		background: var(--sidebar-bg);
		transition: border-color 120ms;
	}
	.card:hover { border-color: var(--muted); }
	.card.orphaned { border-color: rgba(224, 144, 80, 0.3); }

	.card-thumb {
		width: 100%;
		aspect-ratio: 4 / 3;
		overflow: hidden;
		background: var(--bg);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: zoom-in;
		position: relative;
		border: none;
		padding: 0;
	}

	.drawing-thumb :global(svg) {
		width: 100%;
		height: 100%;
		padding: 0.5rem;
	}

	.thumb-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0;
		transition: opacity 120ms;
		pointer-events: none;
	}
	.thumb-overlay svg {
		width: 28px;
		height: 28px;
		color: white;
		filter: drop-shadow(0 1px 3px rgba(0,0,0,0.6));
	}
	.card-thumb:hover .thumb-overlay { opacity: 1; }

	.thumb-icon {
		font-size: 2rem;
		color: var(--muted);
	}

	.card-footer {
		display: flex;
		align-items: center;
		padding: 0.45rem 0.6rem;
		gap: 0.4rem;
		border-top: 1px solid var(--border);
	}

	.card-info {
		flex: 1;
		min-width: 0;
	}

	.card-name {
		font-size: 0.78rem;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		display: block;
	}

	.badge-orphaned {
		font-size: 0.65rem;
		padding: 0.1rem 0.35rem;
		border-radius: 4px;
		background: rgba(224, 144, 80, 0.1);
		border: 1px solid rgba(224, 144, 80, 0.35);
		color: #e09050;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.card-delete {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		font-size: 0.72rem;
		padding: 0.2rem 0.3rem;
		border-radius: 4px;
		flex-shrink: 0;
		line-height: 1;
		transition: color 80ms, background 80ms;
	}
	.card-delete:hover { color: #e57373; background: rgba(229, 115, 115, 0.1); }
</style>
