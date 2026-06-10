<script lang="ts">
	import { onMount } from 'svelte';
	import { getSyncStatus, triggerSync, type SyncStatus } from './api';

	let status = $state<SyncStatus | null>(null);
	let syncing = $state(false);
	let open = $state(false);

	onMount(() => {
		getSyncStatus().then(s => { status = s; }).catch(() => {});
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short',
		});
	}

	async function onSyncNow() {
		if (syncing) return;
		syncing = true;
		try {
			await triggerSync();
			// Poll until the status updates (backend runs async)
			await new Promise(r => setTimeout(r, 1200));
			status = await getSyncStatus();
		} catch {
			// ignore — status will show the error
		} finally {
			syncing = false;
		}
	}
</script>

<section>
	<button class="section-toggle" onclick={() => (open = !open)}>
		<span>Git Sync</span>
		<svg class="chevron" class:open viewBox="0 0 10 6" width="10" height="6" aria-hidden="true">
			<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>

	{#if open}
	<div class="section-content">
		{#if status === null}
			<p class="section-desc">Loading…</p>
		{:else if !status.configured}
			<p class="section-desc">
				Git sync is not configured for this partition. Add a <code>[&lt;slug&gt;.sync]</code> section
				to the root <code>partition.toml</code> and the token to
				<code>clef-note.toml</code>, then restart the server.
			</p>
			<p class="section-desc example-label">In <code>partition.toml</code> (at the partitions root):</p>
			<pre class="example">[notes.sync]
remote = "https://github.com/you/notes.git"
branch = "main"
interval_minutes = 30
# author_name  = "clef-note"
# author_email = "sync@local"</pre>
			<p class="section-desc example-label">In <code>clef-note.toml</code> (outside all partitions — never committed):</p>
			<pre class="example">[partition_git_tokens]
notes = "ghp_xxxx"   # key = partition slug</pre>
		{:else}
			<div class="status-block">
				<div class="status-row">
					<span class="status-label">Last sync</span>
					<span class="status-value">
						{status.last_sync_at ? formatDate(status.last_sync_at) : 'Never'}
					</span>
				</div>
				{#if status.last_error}
				<div class="status-row error-row">
					<span class="status-label">Error</span>
					<span class="status-value error-text">{status.last_error}</span>
				</div>
				{/if}
			</div>

			<button class="sync-btn" onclick={onSyncNow} disabled={syncing}>
				{syncing ? 'Syncing…' : 'Sync now'}
			</button>

			<p class="section-desc">
				Storage is synced to the configured remote on every interval and at startup.
				Conflicts keep the local version and create a <code>Conflict - …</code> note for review.
			</p>
		{/if}
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
		line-height: 1.5;
	}

	.example-label {
		margin: 0;
	}

	section code {
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.8em;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.05em 0.3em;
	}

	.example {
		margin: 0;
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.78rem;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem 1rem;
		color: var(--text);
		overflow-x: auto;
		line-height: 1.6;
	}

	.status-block {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.6rem 0.9rem;
	}

	.status-row {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
	}

	.status-label {
		font-size: 0.78rem;
		color: var(--muted);
		flex-shrink: 0;
		min-width: 68px;
	}

	.status-value {
		font-size: 0.82rem;
		color: var(--text);
	}

	.error-row { margin-top: 0.2rem; }

	.error-text {
		color: #e57373;
		font-size: 0.78rem;
		word-break: break-all;
	}

	.sync-btn {
		align-self: flex-start;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.8rem;
		font-size: 0.82rem;
		font-family: inherit;
		color: var(--text);
		cursor: pointer;
		transition: background 80ms, border-color 80ms;
	}

	.sync-btn:hover:not(:disabled) {
		background: var(--border);
	}

	.sync-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
