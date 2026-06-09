<script lang="ts">
	import { createVault, deleteVault, switchVault, type VaultInfo } from './api';

	interface Props {
		vaults: VaultInfo[];
		onSwitch: (slug: string) => void;
		onCreated: (vault: VaultInfo) => void;
		onDeleted: (slug: string) => void;
	}

	let { vaults, onSwitch, onCreated, onDeleted }: Props = $props();

	let open = $state(false);
	let creating = $state(false);
	let newName = $state('');
	let createError = $state('');
	let createLoading = $state(false);
	let confirmDelete = $state<string | null>(null);
	let deleteConfirmName = $state('');

	const active = $derived(vaults.find(v => v.active));

	function toggle() { open = !open; }

	function close() {
		open = false;
		creating = false;
		newName = '';
		createError = '';
		confirmDelete = null;
		deleteConfirmName = '';
	}

	async function doSwitch(slug: string) {
		if (slug === active?.slug) { close(); return; }
		try {
			await switchVault(slug);
			onSwitch(slug);
			close();
		} catch {
			// ignore — UI stays open
		}
	}

	async function doCreate() {
		const name = newName.trim();
		if (!name) return;
		createLoading = true;
		createError = '';
		try {
			const vault = await createVault(name);
			onCreated(vault);
			newName = '';
			creating = false;
		} catch (e: unknown) {
			createError = e instanceof Error ? e.message : 'Error creating partition';
		} finally {
			createLoading = false;
		}
	}

	function askDelete(slug: string) {
		confirmDelete = slug;
		deleteConfirmName = '';
	}

	async function doDelete() {
		if (!confirmDelete) return;
		const vault = vaults.find(v => v.slug === confirmDelete);
		if (!vault || deleteConfirmName !== vault.name) return;
		try {
			await deleteVault(confirmDelete);
			onDeleted(confirmDelete);
			confirmDelete = null;
			deleteConfirmName = '';
		} catch {
			confirmDelete = null;
		}
	}

	function focus(el: HTMLElement) { el.focus(); }
</script>

{#if open}
	<button class="backdrop" onclick={close} aria-label="Close"></button>
{/if}

<div class="switcher">
	<button class="trigger" onclick={toggle} aria-haspopup="listbox" aria-expanded={open}>
		<svg class="vault-icon" width="11" height="11" viewBox="0 0 12 12" fill="none" aria-hidden="true">
			<rect x="1" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
			<rect x="7" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
			<rect x="1" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
			<rect x="7" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
		</svg>
		<span class="trigger-name">{active?.name ?? 'Notes'}</span>
		<svg class="chevron" class:open width="8" height="8" viewBox="0 0 8 6" fill="none" aria-hidden="true">
			<path d="M1 1l3 3 3-3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>

	{#if open}
		<div class="dropdown" role="listbox">
			<div class="dropdown-list">
				{#each vaults as vault (vault.slug)}
					<div class="vault-row">
						<button
							class="vault-item"
							class:active={vault.active}
							role="option"
							aria-selected={vault.active}
							onclick={() => doSwitch(vault.slug)}
						>
							<span class="vault-item-name">{vault.name}</span>
							{#if vault.has_sync}
								<svg class="sync-dot" width="8" height="8" viewBox="0 0 10 10" fill="none" aria-label="synced" title="Git sync enabled">
									<circle cx="5" cy="5" r="3.5" stroke="currentColor" stroke-width="1.2"/>
									<path d="M3.5 5l1 1 2-2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
							{#if vault.active}
								<svg class="check" width="10" height="10" viewBox="0 0 10 8" fill="none" aria-hidden="true">
									<path d="M1 4l3 3L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
								</svg>
							{/if}
						</button>
						{#if !vault.active}
							<button
								class="delete-btn"
								onclick={() => askDelete(vault.slug)}
								title="Delete partition"
								aria-label="Delete partition {vault.name}"
							>
								<svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
									<path d="M2 2l6 6M8 2L2 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
								</svg>
							</button>
						{/if}
					</div>
				{/each}
			</div>

			<div class="dropdown-footer">
				{#if creating}
					<form onsubmit={(e) => { e.preventDefault(); doCreate(); }} class="create-form">
						<input
							bind:value={newName}
							placeholder="Partition name"
							use:focus
							onkeydown={(e) => e.key === 'Escape' && (creating = false)}
							class="create-input"
							disabled={createLoading}
						/>
						<button type="submit" class="create-submit" disabled={createLoading || !newName.trim()}>
							{createLoading ? '…' : 'Create'}
						</button>
					</form>
					{#if createError}
						<p class="create-error">{createError}</p>
					{/if}
				{:else}
					<button class="new-btn" onclick={() => { creating = true; }}>
						<svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
							<path d="M5 1v8M1 5h8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
						</svg>
						New partition
					</button>
				{/if}
			</div>
		</div>
	{/if}
</div>

{#if confirmDelete}
	{@const vault = vaults.find(v => v.slug === confirmDelete)}
	{#if vault}
		<div class="confirm-overlay">
			<div class="confirm-dialog">
				<p class="confirm-title">Delete <strong>{vault.name}</strong>?</p>
				<p class="confirm-desc">This will permanently delete all notes, assets and drawings in this partition. Type the partition name to confirm.</p>
				<input
					bind:value={deleteConfirmName}
					placeholder={vault.name}
					use:focus
					class="confirm-input"
					onkeydown={(e) => e.key === 'Escape' && (confirmDelete = null)}
				/>
				<div class="confirm-actions">
					<button class="confirm-cancel" onclick={() => (confirmDelete = null)}>Cancel</button>
					<button
						class="confirm-delete"
						disabled={deleteConfirmName !== vault.name}
						onclick={doDelete}
					>Delete</button>
				</div>
			</div>
		</div>
	{/if}
{/if}

<style>
	.switcher {
		position: relative;
	}

	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 90;
		background: transparent;
		border: none;
		cursor: default;
	}

	.trigger {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0.2rem 0.35rem;
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.7rem;
		font-weight: 600;
		letter-spacing: 0.07em;
		text-transform: uppercase;
		transition: background 80ms, color 80ms;
		max-width: 100%;
		min-width: 0;
	}

	.trigger:hover {
		background: var(--border);
		color: var(--text);
	}

	.vault-icon { flex-shrink: 0; }

	.trigger-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}

	.chevron {
		flex-shrink: 0;
		transition: transform 150ms ease;
		color: var(--muted);
	}

	.chevron.open { transform: rotate(180deg); }

	/* ── Dropdown ─────────────────────────────────── */
	.dropdown {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		min-width: 180px;
		max-width: 260px;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		box-shadow: 0 4px 16px rgba(0,0,0,0.12);
		z-index: 100;
		overflow: hidden;
	}

	.dropdown-list {
		padding: 0.3rem 0;
	}

	.vault-row {
		display: flex;
		align-items: center;
		padding: 0 0.3rem;
	}

	.vault-item {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		background: none;
		border: none;
		padding: 0.4rem 0.5rem;
		font-family: inherit;
		font-size: 0.85rem;
		color: var(--text);
		cursor: pointer;
		border-radius: 5px;
		text-align: left;
		min-width: 0;
		transition: background 80ms;
	}

	.vault-item:hover { background: var(--border); }

	.vault-item.active {
		color: var(--accent);
		font-weight: 500;
	}

	.vault-item-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sync-dot { color: var(--muted); flex-shrink: 0; }
	.check { color: var(--accent); flex-shrink: 0; }

	.delete-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0.25rem;
		border-radius: 4px;
		display: flex;
		align-items: center;
		opacity: 0;
		transition: opacity 80ms, background 80ms;
		flex-shrink: 0;
	}

	.vault-row:hover .delete-btn { opacity: 1; }
	.delete-btn:hover { background: var(--border); color: var(--text); }

	/* ── Footer ───────────────────────────────────── */
	.dropdown-footer {
		border-top: 1px solid var(--border);
		padding: 0.3rem;
	}

	.new-btn {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		background: none;
		border: none;
		padding: 0.4rem 0.5rem;
		font-family: inherit;
		font-size: 0.82rem;
		color: var(--muted);
		cursor: pointer;
		border-radius: 5px;
		transition: background 80ms, color 80ms;
	}

	.new-btn:hover { background: var(--border); color: var(--text); }

	.create-form {
		display: flex;
		gap: 0.3rem;
		padding: 0.15rem 0;
	}

	.create-input {
		flex: 1;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 0.3rem 0.5rem;
		font-size: 0.82rem;
		color: var(--text);
		outline: none;
		font-family: inherit;
		min-width: 0;
	}

	.create-input:focus { border-color: var(--accent); }

	.create-submit {
		background: var(--accent);
		border: none;
		border-radius: 4px;
		padding: 0.3rem 0.6rem;
		font-size: 0.78rem;
		font-family: inherit;
		color: #fff;
		cursor: pointer;
		white-space: nowrap;
		transition: opacity 80ms;
	}

	.create-submit:disabled { opacity: 0.4; cursor: default; }

	.create-error {
		font-size: 0.75rem;
		color: #e57373;
		margin: 0.2rem 0 0;
		padding: 0 0.2rem;
	}

	/* ── Confirm delete ───────────────────────────── */
	.confirm-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.45);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 300;
	}

	.confirm-dialog {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.5rem;
		max-width: 360px;
		width: calc(100% - 2rem);
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		box-shadow: 0 8px 32px rgba(0,0,0,0.2);
	}

	.confirm-title {
		margin: 0;
		font-size: 0.95rem;
		color: var(--text);
	}

	.confirm-desc {
		margin: 0;
		font-size: 0.82rem;
		color: var(--muted);
		line-height: 1.5;
	}

	.confirm-input {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.4rem 0.6rem;
		font-size: 0.85rem;
		color: var(--text);
		outline: none;
		font-family: inherit;
	}

	.confirm-input:focus { border-color: #e57373; }

	.confirm-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.confirm-cancel {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.35rem 0.8rem;
		font-size: 0.85rem;
		font-family: inherit;
		color: var(--text);
		cursor: pointer;
	}

	.confirm-delete {
		background: #e57373;
		border: none;
		border-radius: 6px;
		padding: 0.35rem 0.8rem;
		font-size: 0.85rem;
		font-family: inherit;
		color: #fff;
		cursor: pointer;
		transition: opacity 80ms;
	}

	.confirm-delete:disabled { opacity: 0.4; cursor: default; }
</style>
