<script lang="ts">
	interface Props {
		selected: string;
		saving: boolean;
		saveFailed: boolean;
		isLocked: boolean;
		focusMode: boolean;
		rawView?: boolean;
		isMobile: boolean;
		renaming: boolean;
		renameValue: string;
		renameError?: string;
		onStartRename: () => void;
		onConfirmRename: () => void;
		onCancelRename: () => void;
		onToggleLock: () => void;
		onToggleFocus: () => void;
		onToggleRaw: () => void;
		onOpenPalette: () => void;
	}

	let {
		selected,
		saving,
		saveFailed,
		isLocked,
		focusMode,
		rawView = false,
		isMobile,
		renaming = $bindable(),
		renameValue = $bindable(),
		renameError = '',
		onStartRename,
		onConfirmRename,
		onCancelRename,
		onToggleLock,
		onToggleFocus,
		onToggleRaw,
		onOpenPalette,
	}: Props = $props();

	let renameInput = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (renaming && renameInput) {
			renameInput.focus();
			renameInput.select();
		}
	});

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') { e.preventDefault(); onConfirmRename(); }
		if (e.key === 'Escape') { onCancelRename(); }
	}
</script>

{#if !isMobile}
	<div class="titlebar" role="banner">
		{#if renaming}
			<div class="rename-row">
				<input
					bind:this={renameInput}
					bind:value={renameValue}
					onkeydown={onKeydown}
					onblur={onConfirmRename}
					class="rename-input"
					aria-label="Note name"
					style="width: {renameValue.length + 2}ch;"
				/>
				{#if renameError}
					<span class="rename-error" role="alert">{renameError}</span>
				{/if}
			</div>
		{:else}
			<button onclick={onStartRename} title="Click to rename" class="title-btn">
				{selected}
			</button>
		{/if}

		<div class="titlebar-actions">
			{#if saving}
				<span class="status-label">Saving…</span>
			{:else if saveFailed}
				<span class="status-label status-error" role="alert">Save failed</span>
			{/if}

			<button
				onclick={onToggleFocus}
				title={focusMode ? 'Exit focus mode (Ctrl+Shift+F)' : 'Focus mode (Ctrl+Shift+F)'}
				class="icon-btn"
				class:active={focusMode}
				aria-label={focusMode ? 'Exit focus mode' : 'Focus mode'}
				aria-pressed={focusMode}
			>
				<svg width="14" height="14" viewBox="0 0 20 20" fill="none" aria-hidden="true">
					{#if focusMode}
						<path d="M3 8V3h5M17 8V3h-5M3 12v5h5M17 12v5h-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
					{:else}
						<path d="M7 3H3v4M13 3h4v4M7 17H3v-4M13 17h4v-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
					{/if}
				</svg>
			</button>

			<button
				onclick={onToggleRaw}
				title={rawView ? 'Show formatted view' : 'View markdown source'}
				class="icon-btn"
				class:active={rawView}
				aria-label={rawView ? 'Show formatted view' : 'View markdown source'}
				aria-pressed={rawView}
			>
				<svg width="15" height="15" viewBox="0 0 20 20" fill="none" aria-hidden="true">
					<path d="M7.5 6.5L4 10l3.5 3.5M12.5 6.5L16 10l-3.5 3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
				</svg>
			</button>

			<button
				onclick={onToggleLock}
				title={isLocked ? 'Unlock note' : 'Lock note (read-only)'}
				class="icon-btn"
				class:locked={isLocked}
				aria-label={isLocked ? 'Unlock note' : 'Lock note'}
				aria-pressed={isLocked}
			>
				{#if isLocked}
					<svg width="14" height="14" viewBox="0 0 20 20" fill="none" aria-hidden="true">
						<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
						<path d="M7 9V6a3 3 0 0 1 6 0v3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				{:else}
					<svg width="14" height="14" viewBox="0 0 20 20" fill="none" aria-hidden="true">
						<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
						<path d="M7 9V6a3 3 0 0 1 6 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				{/if}
			</button>

			<button
				onclick={onOpenPalette}
				title="Commands (Ctrl+K)"
				class="cmd-btn"
				aria-label="Open commands"
				aria-keyshortcuts="Control+K"
			>
				<svg width="13" height="13" viewBox="0 0 20 20" fill="none" aria-hidden="true">
					<circle cx="8.5" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.5"/>
					<path d="M13.5 13.5L17 17" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
				<kbd>Ctrl K</kbd>
			</button>
		</div>
	</div>
{:else if renaming}
	<div class="mobile-rename">
		<input
			bind:this={renameInput}
			bind:value={renameValue}
			onkeydown={onKeydown}
			onblur={onConfirmRename}
			class="rename-input"
			aria-label="Note name"
			style="width: 100%;"
		/>
		{#if renameError}
			<span class="rename-error" role="alert">{renameError}</span>
		{/if}
	</div>
{/if}

<style>
	/* ── Desktop titlebar ──────────────────────────────────── */
	.titlebar {
		padding: 0.4rem 1rem;
		border-bottom: 1px solid var(--border);
		display: flex;
		justify-content: space-between;
		align-items: center;
		flex-shrink: 0;
	}

	.title-btn {
		background: none;
		border: none;
		padding: 0;
		cursor: text;
		font-weight: 600;
		font-size: 1.05rem;
		letter-spacing: -0.02em;
		color: var(--text);
		font-family: inherit;
	}

	.titlebar-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	/* ── Save status ───────────────────────────────────────── */
	.status-label {
		font-size: 0.8rem;
		color: var(--muted);
	}

	.status-error {
		color: var(--color-danger);
	}

	/* ── Icon buttons (focus + lock) ───────────────────────── */
	.icon-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		padding: 0.25rem;
		border-radius: 5px;
		display: flex;
		align-items: center;
		opacity: 0.4;
		transition: opacity 0.15s, color 0.15s;
	}

	.icon-btn:hover { opacity: 1; }
	.icon-btn.active { opacity: 1; color: var(--accent); }
	.icon-btn.locked { opacity: 1; color: var(--color-warning); }

	/* ── Command palette shortcut button ───────────────────── */
	.cmd-btn {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.2rem 0.5rem;
		font-size: 0.75rem;
		color: var(--muted);
		cursor: pointer;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-family: inherit;
	}

	.cmd-btn:hover { background: var(--border); }

	.cmd-btn kbd {
		font-family: inherit;
		font-size: 0.7rem;
	}

	/* ── Rename ────────────────────────────────────────────── */
	.rename-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.rename-input {
		font-weight: 500;
		font-size: 0.95rem;
		background: none;
		border: none;
		border-bottom: 1.5px solid var(--accent);
		outline: none;
		color: var(--text);
		font-family: inherit;
		padding: 0 0.1rem;
		min-width: 8rem;
		max-width: 100%;
	}

	.rename-error {
		font-size: 0.75rem;
		color: var(--color-danger);
	}

	/* ── Mobile rename ─────────────────────────────────────── */
	.mobile-rename {
		padding: 0.4rem 1rem;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
</style>
