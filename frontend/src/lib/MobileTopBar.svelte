<script lang="ts">
	interface Props {
		title: string;
		isLocked: boolean;
		hasNote: boolean;
		onMenu: () => void;
		onToggleLock: () => void;
		onSearch: () => void;
	}

	let { title, isLocked, hasNote, onMenu, onToggleLock, onSearch }: Props = $props();
</script>

<header class="mobile-topbar">
	<button class="topbar-btn" onclick={onMenu} aria-label="Menu">
		<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
			<path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
		</svg>
	</button>

	<span class="topbar-title">{title}</span>

	{#if hasNote}
		<button
			class="topbar-btn"
			class:topbar-locked={isLocked}
			onclick={onToggleLock}
			aria-label={isLocked ? 'Unlock note' : 'Lock note'}
			aria-pressed={isLocked}
		>
			{#if isLocked}
				<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
					<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
					<path d="M7 9V6a3 3 0 0 1 6 0v3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
			{:else}
				<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
					<rect x="4" y="9" width="12" height="9" rx="2" stroke="currentColor" stroke-width="1.5"/>
					<path d="M7 9V6a3 3 0 0 1 6 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
			{/if}
		</button>
	{/if}

	<button class="topbar-btn" onclick={onSearch} aria-label="Search">
		<svg viewBox="0 0 20 20" fill="none" aria-hidden="true">
			<circle cx="8.5" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.5"/>
			<path d="M13.5 13.5L17 17" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
		</svg>
	</button>
</header>

<style>
	.mobile-topbar {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		height: 48px;
		background: var(--sidebar-bg);
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		padding: 0 0.5rem;
		z-index: 100;
		gap: 0.25rem;
	}

	.topbar-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text);
		padding: 0.4rem;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.topbar-btn svg {
		width: 20px;
		height: 20px;
	}

	.topbar-btn:active { background: var(--border); }

	.topbar-locked { color: var(--color-warning); }

	.topbar-title {
		flex: 1;
		font-size: 1.05rem;
		font-weight: 600;
		letter-spacing: -0.02em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		text-align: center;
		padding: 0 0.25rem;
	}
</style>
