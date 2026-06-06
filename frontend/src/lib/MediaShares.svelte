<script lang="ts">
	import { deleteShare, updateShare, type ShareMeta } from './api';

	interface Props {
		shares: ShareMeta[];
		onChanged: () => void;
	}

	let { shares, onChanged }: Props = $props();

	let editing = $state<string | null>(null);
	let editPassword = $state('');
	let editExpiry = $state<number>(7);
	let editShowPw = $state(false);
	let editLoading = $state(false);
	let confirmDelete = $state<string | null>(null);

	const EXPIRY_OPTIONS = [
		{ label: '1 day',     days: 1 },
		{ label: '7 days',    days: 7 },
		{ label: '30 days',   days: 30 },
		{ label: '90 days',   days: 90 },
		{ label: 'No expiry', days: 0 },
	];

	function startEdit(share: ShareMeta) {
		editing = share.slug;
		editPassword = '';
		editShowPw = false;
		// Guess remaining days from expires_at
		if (share.expires_at) {
			const ms = new Date(share.expires_at).getTime() - Date.now();
			const days = Math.max(1, Math.round(ms / 86400000));
			// snap to nearest option
			const closest = [1, 7, 30, 90].reduce((a, b) => Math.abs(b - days) < Math.abs(a - days) ? b : a);
			editExpiry = closest;
		} else {
			editExpiry = 0;
		}
	}

	async function saveEdit(slug: string) {
		editLoading = true;
		try {
			const expiresAt = editExpiry > 0
				? (() => { const d = new Date(); d.setDate(d.getDate() + editExpiry); return d.toISOString(); })()
				: null;
			await updateShare(slug, {
				expires_at: expiresAt,
				...(editPassword ? { password: editPassword } : {}),
			});
			editing = null;
			onChanged();
		} finally {
			editLoading = false;
		}
	}

	async function doDelete(slug: string) {
		await deleteShare(slug);
		confirmDelete = null;
		onChanged();
	}

	function shareUrl(slug: string) {
		return `${window.location.origin}/shared/${slug}`;
	}

	function formatDate(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
	}

	function isExpired(iso: string | null): boolean {
		if (!iso) return false;
		return new Date(iso).getTime() < Date.now();
	}

	async function copyUrl(slug: string) {
		await navigator.clipboard.writeText(shareUrl(slug));
		copiedSlug = slug;
		setTimeout(() => { if (copiedSlug === slug) copiedSlug = null; }, 2000);
	}

	let copiedSlug = $state<string | null>(null);
</script>

{#if shares.length === 0}
	<div class="empty">No shared notes yet.</div>
{:else}
	<div class="shares-list">
		{#each shares as share (share.slug)}
			<div class="share-row" class:expired={isExpired(share.expires_at)}>
				<div class="share-info">
					<span class="share-note">{share.note}</span>
					<div class="share-meta">
						<code class="share-slug">/shared/{share.slug}</code>
						{#if share.has_password}<span class="badge pw-badge">🔒 password</span>{/if}
						{#if isExpired(share.expires_at)}<span class="badge exp-badge">expired</span>{/if}
					</div>
					<div class="share-dates">
						<span>Created {formatDate(share.created_at)}</span>
						<span>·</span>
						<span>Expires {formatDate(share.expires_at)}</span>
					</div>
				</div>

				<div class="share-actions">
					<button class="act-btn copy-btn" class:copied={copiedSlug === share.slug} onclick={() => copyUrl(share.slug)}>
						{copiedSlug === share.slug ? 'Copied!' : 'Copy link'}
					</button>
					<button class="act-btn" onclick={() => startEdit(share)}>Edit</button>
					<button class="act-btn danger-btn" onclick={() => (confirmDelete = share.slug)}>Delete</button>
				</div>

				{#if editing === share.slug}
					<div class="edit-panel">
						<div class="edit-field">
							<span class="edit-label">Expiry</span>
							<div class="expiry-pills">
								{#each EXPIRY_OPTIONS as opt}
									<button class="pill" class:active={editExpiry === opt.days} onclick={() => (editExpiry = opt.days)}>
										{opt.label}
									</button>
								{/each}
							</div>
						</div>
						<div class="edit-field">
							<span class="edit-label">New password <span class="optional">(leave blank to keep current)</span></span>
							<div class="pw-row">
								<input
									class="text-input"
									type={editShowPw ? 'text' : 'password'}
									bind:value={editPassword}
									placeholder="Enter new password…"
									autocomplete="new-password"
								/>
								<button class="show-btn" onclick={() => (editShowPw = !editShowPw)}>
									{editShowPw ? 'Hide' : 'Show'}
								</button>
							</div>
						</div>
						<div class="edit-footer">
							<button class="act-btn" onclick={() => (editing = null)}>Cancel</button>
							<button class="act-btn primary-btn" disabled={editLoading} onclick={() => saveEdit(share.slug)}>
								{editLoading ? 'Saving…' : 'Save'}
							</button>
						</div>
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}

{#if confirmDelete}
	<div class="confirm-backdrop" role="dialog" aria-modal="true">
		<div class="confirm-box">
			<p>Delete share <code>/shared/{confirmDelete}</code>?</p>
			<p class="confirm-sub">The link will stop working immediately.</p>
			<div class="confirm-actions">
				<button class="act-btn" onclick={() => (confirmDelete = null)}>Cancel</button>
				<button class="act-btn danger-btn" onclick={() => doDelete(confirmDelete!)}>Delete</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.empty {
		color: var(--muted);
		font-size: 0.9rem;
		text-align: center;
		padding: 3rem 0;
	}

	.shares-list {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.share-row {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid var(--border);
		transition: background 80ms;
	}

	.share-row:hover { background: color-mix(in srgb, var(--accent) 4%, var(--bg)); }
	.share-row.expired { opacity: 0.6; }

	.share-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.share-note {
		font-weight: 600;
		font-size: 0.9rem;
		color: var(--text);
	}

	.share-meta {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
	}

	.share-slug {
		font-size: 0.78rem;
		color: var(--accent);
		background: color-mix(in srgb, var(--accent) 8%, var(--bg));
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
	}

	.badge {
		font-size: 0.72rem;
		padding: 0.1rem 0.45rem;
		border-radius: 10px;
		font-weight: 500;
	}

	.pw-badge {
		background: color-mix(in srgb, var(--muted) 15%, var(--bg));
		color: var(--muted);
	}

	.exp-badge {
		background: color-mix(in srgb, var(--color-danger) 15%, var(--bg));
		color: var(--color-danger);
	}

	.share-dates {
		display: flex;
		gap: 0.4rem;
		font-size: 0.76rem;
		color: var(--muted);
	}

	.share-actions {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}

	.act-btn {
		padding: 0.25rem 0.65rem;
		border-radius: 5px;
		border: 1px solid var(--border);
		background: var(--bg);
		color: var(--text);
		font-size: 0.78rem;
		cursor: pointer;
		font-family: inherit;
		white-space: nowrap;
		transition: border-color 80ms;
	}

	.act-btn:hover { border-color: var(--accent); }
	.act-btn:disabled { opacity: 0.55; cursor: default; }

	.copy-btn.copied {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}

	.danger-btn:hover { border-color: var(--color-danger); color: var(--color-danger); }

	.primary-btn {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}

	/* ── Edit panel ── */
	.edit-panel {
		border-top: 1px solid var(--border);
		padding-top: 0.8rem;
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
	}

	.edit-field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.edit-label {
		font-size: 0.76rem;
		font-weight: 600;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.optional { font-weight: 400; text-transform: none; letter-spacing: 0; }

	.expiry-pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	.pill {
		padding: 0.25rem 0.65rem;
		border-radius: 20px;
		border: 1px solid var(--border);
		background: var(--bg);
		color: var(--text);
		font-size: 0.78rem;
		cursor: pointer;
		font-family: inherit;
	}

	.pill.active {
		background: var(--accent);
		border-color: var(--accent);
		color: #fff;
	}

	.pw-row {
		display: flex;
		gap: 0.4rem;
	}

	.text-input {
		flex: 1;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.6rem;
		font-size: 0.85rem;
		font-family: inherit;
		color: var(--text);
		outline: none;
	}

	.text-input:focus { border-color: var(--accent); }

	.show-btn {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.6rem;
		font-size: 0.78rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
	}

	.edit-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
	}

	/* ── Confirm dialog ── */
	.confirm-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.45);
		z-index: 400;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.confirm-box {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.5rem;
		max-width: 360px;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.confirm-box p { margin: 0; font-size: 0.9rem; color: var(--text); }
	.confirm-sub { color: var(--muted) !important; font-size: 0.82rem !important; }

	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
		margin-top: 0.5rem;
	}
</style>
