<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchKeys } from './api';

	let open = $state(false);
	let apiKey = $state('');
	let apiKeyRevealed = $state(false);
	let apiKeyCopied = $state(false);

	onMount(() => {
		fetchKeys().then(k => { apiKey = k.api_key; }).catch(() => {});
	});

	function copyApiKey() {
		navigator.clipboard.writeText(apiKey);
		apiKeyCopied = true;
		setTimeout(() => { apiKeyCopied = false; }, 2000);
	}

	function masked(k: string) {
		return k.slice(0, 8) + '·'.repeat(24) + k.slice(-8);
	}
</script>

<section>
	<button class="section-toggle" onclick={() => (open = !open)}>
		<span>Security</span>
		<svg class="chevron" class:open viewBox="0 0 10 6" width="10" height="6" aria-hidden="true">
			<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>
	{#if open}
	<div class="section-content">
		<p class="section-desc">Keys are defined in <code>clef-note.toml</code>. To rotate, update the file and restart the backend.</p>

		<div class="key-block">
			<div class="key-label">
				<span>API key</span>
				<span class="key-hint">CLI · REST · OpenAPI (<code>AN_KEY</code>)</span>
			</div>
			{#if apiKey}
				<div class="key-row">
					<code class="key-display">{apiKeyRevealed ? apiKey : masked(apiKey)}</code>
					<button class="option-btn" onclick={() => { apiKeyRevealed = !apiKeyRevealed; }}>
						{apiKeyRevealed ? 'Hide' : 'Reveal'}
					</button>
					<button class="option-btn key-copy" class:copied={apiKeyCopied} onclick={copyApiKey}>
						{apiKeyCopied ? 'Copied!' : 'Copy'}
					</button>
				</div>
			{/if}
		</div>
	</div>
	{/if}
</section>

<style>
	section {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.section-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		background: none;
		border: none;
		border-bottom: 1px solid var(--border);
		padding: 0 0 0.5rem;
		cursor: pointer;
		font-family: inherit;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		transition: color 80ms;
	}

	.section-toggle:hover { color: var(--text); }

	.chevron {
		color: var(--muted);
		transition: transform 150ms ease, color 80ms;
		flex-shrink: 0;
		transform: rotate(-90deg);
	}

	.chevron.open { transform: rotate(0deg); }

	.section-toggle:hover .chevron { color: var(--text); }

	.section-content {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding-top: 1rem;
	}

	.section-desc {
		margin: -0.5rem 0 0;
		font-size: 0.8rem;
		color: var(--muted);
	}

	section code {
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.8em;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.05em 0.3em;
	}

	.key-block {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.key-label {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--text);
	}

	.key-hint {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--muted);
	}

	.key-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.45rem 0.75rem;
	}

	.key-display {
		flex: 1;
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.75rem;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		letter-spacing: 0.02em;
		min-width: 0;
	}

	.option-btn {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.65rem;
		font-size: 0.82rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
		transition: background 80ms, border-color 80ms, color 80ms;
		white-space: nowrap;
	}

	.option-btn:hover { background: var(--border); color: var(--text); }

	.key-copy.copied {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg);
	}
</style>
