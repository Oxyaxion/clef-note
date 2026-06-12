<script lang="ts">
	import type { NoteMeta } from './api';

	interface Props {
		notes: NoteMeta[];
		onNavigate: (name: string) => void;
	}

	let { notes, onNavigate }: Props = $props();

	function displayName(note: NoteMeta): string {
		return note.name.split('/').pop() || note.name;
	}

	function formatDate(secs: number | undefined): string {
		if (!secs) return '—';
		return new Date(secs * 1000).toLocaleDateString(undefined, {
			year: 'numeric', month: 'short', day: 'numeric',
		});
	}
</script>

{#if notes.length === 0}
	<div class="empty-state">All notes have frontmatter 🎉</div>
{:else}
	<ul class="list">
		<li class="item header">
			<span class="col-name">Note</span>
			<span class="col-date">Modified</span>
		</li>
		{#each notes as note (note.name)}
			<li class="item">
				<button class="note-name" onclick={() => onNavigate(note.name)}>
					{displayName(note)}
				</button>
				{#if note.name !== displayName(note)}
					<span class="note-path">{note.name}</span>
				{/if}
				<span class="col-date">{formatDate(note.modified_at)}</span>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.header {
		background: var(--bg-alt);
		font-size: 0.75rem;
		color: var(--text-muted);
		font-weight: 600;
		border-bottom: 1px solid var(--border);
	}

	.item {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.35rem 1rem;
		border-bottom: 1px solid var(--border);
		transition: background 0.1s;
	}

	.item:last-child { border-bottom: none; }
	.item:not(.header):hover { background: var(--bg-alt); }

	.col-name { flex: 1; }

	.note-name {
		background: none;
		border: none;
		padding: 0;
		font-size: 0.875rem;
		color: var(--accent);
		cursor: pointer;
		text-align: left;
		flex-shrink: 0;
	}

	.note-name:hover { text-decoration: underline; }

	.note-path {
		font-size: 0.75rem;
		color: var(--text-muted);
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.col-date {
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
