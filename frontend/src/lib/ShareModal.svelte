<script lang="ts">
	import { createShare } from './api';

	interface Props {
		noteName: string;
		onClose: () => void;
		onCreated?: () => void;
	}

	let { noteName, onClose, onCreated }: Props = $props();

	// ── Slug ─────────────────────────────────────────────────────────────────
	function generateSlug(name: string): string {
		const base = name
			.split('/')
			.pop()!
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '')
			.slice(0, 32);
		const rand = Math.random().toString(16).slice(2, 8);
		return `${base}-${rand}`;
	}

	let slug = $state(generateSlug(noteName));
	let slugError = $state('');

	function validateSlug(s: string): string {
		if (!s) return 'Required';
		if (!/^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$/.test(s)) return 'Alphanumeric + hyphens only';
		if (s.length > 80) return 'Max 80 characters';
		return '';
	}

	// ── Expiry ────────────────────────────────────────────────────────────────
	const EXPIRY_OPTIONS = [
		{ label: '1 day',    days: 1 },
		{ label: '7 days',   days: 7 },
		{ label: '30 days',  days: 30 },
		{ label: '90 days',  days: 90 },
		{ label: 'No expiry', days: 0 },
	];
	let expiryDays = $state(7);

	function expiryDate(): string | null {
		if (expiryDays === 0) return null;
		const d = new Date();
		d.setDate(d.getDate() + expiryDays);
		return d.toISOString();
	}

	// ── Password ──────────────────────────────────────────────────────────────
	let password = $state('');
	let showPassword = $state(false);

	// ── Submit ────────────────────────────────────────────────────────────────
	let loading = $state(false);
	let error = $state('');
	let createdSlug = $state<string | null>(null);

	const shareUrl = $derived(
		createdSlug ? `${window.location.origin}/shared/${createdSlug}` : ''
	);

	async function submit() {
		slugError = validateSlug(slug);
		if (slugError) return;

		loading = true;
		error = '';
		try {
			await createShare({
				slug,
				note: noteName,
				expires_at: expiryDate(),
				password: password || undefined,
			});
			createdSlug = slug;
			onCreated?.();
		} catch (e: unknown) {
			error = e instanceof Error && e.message === 'slug-conflict'
				? 'This slug is already in use — try another one.'
				: 'Could not create share. Try again.';
		} finally {
			loading = false;
		}
	}

	async function copyLink() {
		await navigator.clipboard.writeText(shareUrl);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	let copied = $state(false);
	let curlPopover = $state(false);

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="backdrop" onclick={onBackdropClick} role="dialog" aria-modal="true" aria-label="Share note">
	<div class="modal">
		<div class="modal-header">
			<span class="modal-title">Share note</span>
			<button class="close-btn" onclick={onClose} aria-label="Close">
				<svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
					<path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
			</button>
		</div>

		{#if createdSlug}
			<!-- ── Success state ── -->
			<div class="success-body">
				<p class="success-label">Share link created</p>
				<div class="link-row">
					<input class="link-input" type="text" readonly value={shareUrl} onclick={(e) => (e.target as HTMLInputElement).select()} />
					<button class="copy-btn" onclick={copyLink} class:copied>
						{copied ? 'Copied!' : 'Copy'}
					</button>
					<div class="curl-wrap">
						<button class="curl-btn" onclick={() => (curlPopover = !curlPopover)} title="curl command">
							curl
						</button>
						{#if curlPopover}
							<div class="curl-popover">
								<code class="curl-code">curl "{window.location.origin}/api/shared/{createdSlug}?raw=1"{password ? ` \\\n  -H "X-Share-Password: ${password}"` : ''}</code>
							</div>
						{/if}
					</div>
				</div>
				{#if expiryDays > 0}
					<p class="hint">Expires in {expiryDays} day{expiryDays > 1 ? 's' : ''}</p>
				{:else}
					<p class="hint">No expiry</p>
				{/if}
				{#if password}
					<p class="hint">Password protected</p>
				{/if}
			</div>
		{:else}
			<!-- ── Form ── -->
			<div class="modal-body">
				<div class="field">
					<label class="field-label" for="share-slug">URL slug</label>
					<div class="slug-row">
						<span class="slug-prefix">/shared/</span>
						<input
							id="share-slug"
							class="slug-input"
							class:invalid={!!slugError}
							type="text"
							bind:value={slug}
							oninput={() => (slugError = '')}
							spellcheck="false"
							autocomplete="off"
						/>
					</div>
					{#if slugError}
						<p class="field-error">{slugError}</p>
					{/if}
				</div>

				<div class="field">
					<label class="field-label">Expiry</label>
					<div class="expiry-pills">
						{#each EXPIRY_OPTIONS as opt}
							<button
								class="pill"
								class:active={expiryDays === opt.days}
								onclick={() => (expiryDays = opt.days)}
							>{opt.label}</button>
						{/each}
					</div>
				</div>

				<div class="field">
					<label class="field-label" for="share-password">Password <span class="optional">(optional)</span></label>
					<div class="pw-row">
						<input
							id="share-password"
							class="text-input"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							placeholder="Leave empty for no password"
							autocomplete="new-password"
						/>
						<button class="show-btn" onclick={() => (showPassword = !showPassword)} aria-label={showPassword ? 'Hide' : 'Show'}>
							{showPassword ? 'Hide' : 'Show'}
						</button>
					</div>
				</div>

				{#if error}
					<p class="form-error">{error}</p>
				{/if}
			</div>

			<div class="modal-footer">
				<button class="btn-secondary" onclick={onClose}>Cancel</button>
				<button class="btn-primary" onclick={submit} disabled={loading}>
					{loading ? 'Creating…' : 'Create share link'}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 300;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.modal {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		width: 100%;
		max-width: 440px;
		box-shadow: 0 8px 40px rgba(0, 0, 0, 0.3);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.2rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	.modal-title {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text);
	}

	.close-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0.2rem;
		border-radius: 4px;
		display: flex;
	}

	.close-btn svg { width: 16px; height: 16px; }
	.close-btn:hover { color: var(--text); }

	/* ── Form body ── */
	.modal-body {
		padding: 1.2rem;
		display: flex;
		flex-direction: column;
		gap: 1.2rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}

	.field-label {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.optional {
		font-weight: 400;
		text-transform: none;
		letter-spacing: 0;
	}

	.slug-row {
		display: flex;
		align-items: center;
		border: 1px solid var(--border);
		border-radius: 6px;
		overflow: hidden;
		background: var(--bg);
	}

	.slug-row:focus-within { border-color: var(--accent); }

	.slug-prefix {
		padding: 0.35rem 0.5rem 0.35rem 0.7rem;
		font-size: 0.82rem;
		color: var(--muted);
		background: var(--sidebar-bg);
		border-right: 1px solid var(--border);
		white-space: nowrap;
		user-select: none;
	}

	.slug-input {
		flex: 1;
		border: none;
		outline: none;
		background: var(--bg);
		color: var(--text);
		font-family: inherit;
		font-size: 0.88rem;
		padding: 0.35rem 0.7rem;
	}

	.slug-input.invalid { background: color-mix(in srgb, var(--color-danger) 8%, var(--bg)); }

	.field-error {
		font-size: 0.78rem;
		color: var(--color-danger);
		margin: 0;
	}

	.expiry-pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}

	.pill {
		padding: 0.3rem 0.75rem;
		border-radius: 20px;
		border: 1px solid var(--border);
		background: var(--bg);
		color: var(--text);
		font-size: 0.82rem;
		cursor: pointer;
		font-family: inherit;
		transition: background 100ms, border-color 100ms;
	}

	.pill.active, .pill:hover {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}

	.pw-row {
		display: flex;
		gap: 0.5rem;
	}

	.text-input {
		flex: 1;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.35rem 0.7rem;
		font-size: 0.88rem;
		font-family: inherit;
		color: var(--text);
		outline: none;
	}

	.text-input:focus { border-color: var(--accent); }

	.show-btn {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.35rem 0.7rem;
		font-size: 0.82rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
		white-space: nowrap;
	}

	.show-btn:hover { color: var(--text); }

	.form-error {
		font-size: 0.82rem;
		color: var(--color-danger);
		margin: -0.4rem 0 0;
	}

	/* ── Footer ── */
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.6rem;
		padding: 0.75rem 1.2rem 1rem;
		border-top: 1px solid var(--border);
	}

	.btn-primary, .btn-secondary {
		padding: 0.4rem 1rem;
		border-radius: 6px;
		font-size: 0.86rem;
		font-family: inherit;
		cursor: pointer;
		border: 1px solid transparent;
	}

	.btn-primary {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}

	.btn-primary:disabled { opacity: 0.55; cursor: default; }
	.btn-primary:not(:disabled):hover { filter: brightness(1.1); }

	.btn-secondary {
		background: var(--bg);
		color: var(--text);
		border-color: var(--border);
	}

	.btn-secondary:hover { border-color: var(--text); }

	/* ── Success ── */
	.success-body {
		padding: 1.2rem;
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
	}

	.success-label {
		font-size: 0.88rem;
		font-weight: 600;
		color: var(--text);
		margin: 0;
	}

	.link-row {
		display: flex;
		gap: 0.5rem;
	}

	.link-input {
		flex: 1;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.35rem 0.7rem;
		font-size: 0.82rem;
		font-family: monospace;
		color: var(--text);
		outline: none;
		min-width: 0;
	}

	.copy-btn {
		padding: 0.35rem 0.9rem;
		border-radius: 6px;
		border: 1px solid var(--border);
		background: var(--bg);
		color: var(--text);
		font-size: 0.82rem;
		cursor: pointer;
		font-family: inherit;
		white-space: nowrap;
		transition: background 120ms;
	}

	.copy-btn.copied {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}

	.hint {
		font-size: 0.78rem;
		color: var(--muted);
		margin: 0;
	}

	.curl-wrap {
		position: relative;
		flex-shrink: 0;
	}

	.curl-btn {
		padding: 0.35rem 0.65rem;
		border-radius: 6px;
		border: 1px solid var(--border);
		background: var(--bg);
		color: var(--muted);
		font-size: 0.78rem;
		font-family: monospace;
		cursor: pointer;
		white-space: nowrap;
	}

	.curl-btn:hover { color: var(--text); border-color: var(--text); }

	.curl-popover {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.6rem 0.75rem;
		box-shadow: 0 4px 20px rgba(0,0,0,0.15);
		z-index: 10;
		width: max-content;
		max-width: min(360px, 90vw);
	}

	.curl-code {
		font-family: monospace;
		font-size: 0.76rem;
		color: var(--text);
		white-space: pre-wrap;
		word-break: break-all;
		display: block;
		margin: 0;
	}
</style>
