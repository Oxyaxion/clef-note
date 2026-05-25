<script lang="ts">
	import { untrack } from 'svelte';
	import { applySettings, DEFAULT, type AppSettings } from './settings';
	import { putSettings } from './api';
	import { type ThemeId } from './theme';
	import { debounce } from './utils';
	import SettingsGeneral from './SettingsGeneral.svelte';
	import SettingsAppearance from './SettingsAppearance.svelte';
	import SettingsSecurity from './SettingsSecurity.svelte';

	interface Props {
		currentTheme: ThemeId;
		onClose: () => void;
		onSetTheme: (id: ThemeId) => void;
		onLogout: () => void;
		onSettingsChange?: (s: AppSettings) => void;
		initialSettings?: AppSettings;
	}

	let { currentTheme, onClose, onSetTheme, onLogout, onSettingsChange, initialSettings = DEFAULT }: Props = $props();

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

		<div class="modal-header">
			<span class="modal-title">Settings</span>
			<button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
		</div>

		<div class="modal-body">
			<SettingsGeneral bind:settings {onChange} />
			<SettingsAppearance bind:settings {currentTheme} {onSetTheme} {onChange} {onReset} />
			<SettingsSecurity />
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
			border-radius: 16px 16px 0 0;
			max-height: 90vh;
			padding-bottom: env(safe-area-inset-bottom, 0);
		}
	}
</style>
