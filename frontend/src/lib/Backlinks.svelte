<script lang="ts">
	import { getBacklinks } from './api';
	import { isAbortError } from './utils';

	interface Props {
		note: string;
		onNavigate: (name: string) => void;
	}

	let { note, onNavigate }: Props = $props();

	let backlinks = $state<string[]>([]);
	let open = $state(false);

	$effect(() => {
		const n = note;
		open = false;
		const ctrl = new AbortController();
		getBacklinks(n, ctrl.signal)
			.then((r) => { backlinks = r.backlinks; })
			.catch((e) => { if (!isAbortError(e)) backlinks = []; });
		return () => { ctrl.abort(); };
	});

	function label(name: string): string {
		return name.split('/').pop() ?? name;
	}

	function folder(name: string): string {
		const parts = name.split('/');
		return parts.length > 1 ? parts.slice(0, -1).join('/') + '/' : '';
	}
</script>

{#if backlinks.length > 0}
	<aside class="backlinks">
		<button class="bl-toggle" onclick={() => (open = !open)} aria-expanded={open}>
			<svg class="bl-chevron" class:open width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
				<path d="M3 4.5l3 3 3-3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
			<svg width="11" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
				<path d="M5 3l-3 4 3 4M9 3l3 4-3 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
			{backlinks.length} backlink{backlinks.length > 1 ? 's' : ''}
		</button>

		{#if open}
			<ul>
				{#each backlinks as bl (bl)}
					<li>
						<button onclick={() => onNavigate(bl)} title={bl}>
							{#if folder(bl)}<span class="bl-folder">{folder(bl)}</span>{/if}<span class="bl-name">{label(bl)}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</aside>
{/if}

<style>
	.backlinks {
		flex-shrink: 0;
		padding: 0.5rem 3rem 0.7rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-top: 1px solid var(--border);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.bl-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		background: none;
		border: none;
		padding: 0.1rem 0.25rem;
		font: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--muted);
		cursor: pointer;
		white-space: nowrap;
		flex-shrink: 0;
		transition: color 80ms;
	}

	.bl-toggle:hover {
		color: var(--text);
	}

	.bl-chevron {
		transition: transform 160ms ease;
		transform: rotate(-90deg);
	}

	.bl-chevron.open {
		transform: rotate(0deg);
	}

	ul button {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
		background: color-mix(in srgb, var(--accent) 8%, transparent);
		border: none;
		border-radius: 4px;
		padding: 0.15rem 0.5rem;
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
		color: var(--accent);
		transition: background 80ms;
	}

	ul button:hover {
		background: color-mix(in srgb, var(--accent) 16%, transparent);
	}

	.bl-folder {
		color: color-mix(in srgb, var(--accent) 55%, var(--muted));
		font-size: 0.75rem;
	}

	.bl-name {
		color: var(--accent);
	}
</style>
