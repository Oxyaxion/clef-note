<script lang="ts">
	import { untrack } from 'svelte';
	import { applySettings, DEFAULT, type AppSettings } from './settings';
	import { putSettings } from './api';
	import { type ThemeId } from './theme';
	import { debounce } from './utils';
	import SettingsGeneral from './SettingsGeneral.svelte';
	import SettingsAppearance from './SettingsAppearance.svelte';
	import SettingsSecurity from './SettingsSecurity.svelte';
	import SettingsSync from './SettingsSync.svelte';

	interface Props {
		currentTheme: ThemeId;
		activePartitionSlug?: string;
		onClose: () => void;
		onSetTheme: (id: ThemeId) => void;
		onLogout: () => void;
		onSettingsChange?: (s: AppSettings) => void;
		initialSettings?: AppSettings;
	}

	let { currentTheme, activePartitionSlug = '', onClose, onSetTheme, onLogout, onSettingsChange, initialSettings = DEFAULT }: Props = $props();

	let settings = $state<AppSettings>({ ...untrack(() => initialSettings) });
	const debouncedSave = debounce(() => putSettings(settings), 400);

	function focusTrap(el: HTMLElement) {
		el.focus();
	}

	function onChange() {
		applySettings(settings);
		onSettingsChange?.(settings);
		debouncedSave();
	}

	function onReset() {
		settings = { ...DEFAULT };
		applySettings(settings);
		onSettingsChange?.(settings);
		putSettings(settings);
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={onBackdropClick}>
	<div class="modal" role="dialog" aria-label="Settings" aria-modal="true" use:focusTrap tabindex="-1">

		<button class="grabber" onclick={onClose} aria-label="Close"></button>

		<div class="modal-header">
			<span class="modal-title">Settings</span>
			<button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
		</div>

		<div class="modal-body">
			<SettingsGeneral bind:settings {activePartitionSlug} {onChange} />
			<SettingsAppearance bind:settings {currentTheme} {onSetTheme} {onChange} {onReset} />
			<SettingsSecurity />
			<SettingsSync />
		</div>

		<div class="modal-footer">
			<button class="signout-btn" onclick={onLogout}>Sign out</button>
			<span class="footer-hint">Changes apply immediately</span>
			<a class="version-hint" href="https://github.com/Oxyaxion/clef-note/releases" target="_blank" rel="noopener noreferrer">{__APP_VERSION__}</a>
		</div>

	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.35);
		backdrop-filter: blur(2px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
		padding: 1rem;
		animation: backdrop-in 180ms ease;
	}

	.modal {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 14px;
		box-shadow: 0 24px 64px rgba(0, 0, 0, 0.28);
		width: 540px;
		max-width: 100%;
		max-height: 88vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		outline: none;
		animation: modal-in 180ms ease;
	}

	@keyframes backdrop-in {
		from { opacity: 0; }
	}

	@keyframes modal-in {
		from { opacity: 0; transform: scale(0.97); }
	}

	@keyframes sheet-in {
		from { transform: translateY(100%); }
	}

	/* ── Grabber (mobile bottom-sheet handle) ────────────── */
	.grabber {
		display: none;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.25rem 0.75rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.modal-title {
		font-weight: 650;
		font-size: 1rem;
		letter-spacing: -0.02em;
	}

	.close-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--muted);
		font-size: 0.85rem;
		padding: 0.25rem 0.4rem;
		border-radius: 5px;
		line-height: 1;
	}

	.close-btn:hover {
		background: var(--border);
		color: var(--text);
	}

	.modal-body {
		overflow-y: auto;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1.75rem;
		flex: 1;
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1.25rem;
		border-top: 1px solid var(--border);
		flex-shrink: 0;
	}

	.footer-hint {
		font-size: 0.75rem;
		color: var(--muted);
	}

	.version-hint {
		font-size: 0.7rem;
		color: var(--muted);
		opacity: 0.5;
		font-variant-numeric: tabular-nums;
		text-decoration: none;
	}

	.version-hint:hover {
		opacity: 0.9;
		text-decoration: underline;
	}

	.signout-btn {
		background: none;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.25rem 0.65rem;
		font-size: 0.78rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
	}

	.signout-btn:hover { border-color: #e57373; color: #e57373; }

	@media (max-width: 640px) {
		.backdrop {
			align-items: flex-end;
			padding: 0;
		}

		.modal {
			width: 100%;
			max-width: 100%;
			border-radius: 20px 20px 0 0;
			max-height: 92vh;
			animation: sheet-in 280ms cubic-bezier(0.32, 0.72, 0, 1);
		}

		/* poignée de préhension */
		.grabber {
			display: block;
			width: 100%;
			padding: 0;
			margin: 0;
			height: 22px;
			position: relative;
			background: none;
			border: none;
			cursor: pointer;
			flex-shrink: 0;
		}

		.grabber::before {
			content: '';
			position: absolute;
			top: 9px;
			left: 50%;
			transform: translateX(-50%);
			width: 38px;
			height: 4px;
			border-radius: 999px;
			background: var(--border);
		}

		.modal-header {
			padding: 0.25rem 1rem 0.75rem;
		}

		.modal-title {
			font-size: 1.1rem;
		}

		/* cible tactile confortable */
		.close-btn {
			font-size: 1rem;
			padding: 0.5rem 0.7rem;
		}

		.modal-body {
			padding: 1.1rem 1.25rem;
			gap: 2rem;
		}

		.modal-footer {
			flex-wrap: wrap;
			gap: 0.6rem 0.75rem;
			padding: 0.85rem 1.25rem;
			padding-bottom: calc(0.85rem + env(safe-area-inset-bottom, 0));
		}

		/* Sign out devient l'action principale, pleine largeur */
		.signout-btn {
			order: 1;
			flex: 1 1 100%;
			text-align: center;
			padding: 0.7rem;
			font-size: 0.9rem;
			border-radius: 8px;
		}

		.footer-hint { order: 2; }
		.version-hint { order: 3; margin-left: auto; }
	}

	@media (prefers-reduced-motion: reduce) {
		.backdrop,
		.modal {
			animation: none;
		}
	}
</style>
