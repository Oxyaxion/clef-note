<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { THEMES, type ThemeId } from './theme';
	import { applySettings, DEFAULT, FONT_PRESETS, DATE_FORMATS, type AppSettings } from './settings';
	import { fetchKeys, putSettings } from './api';
	import { debounce } from './utils';

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

	let apiKey = $state('');
	let apiKeyRevealed = $state(false);
	let apiKeyCopied = $state(false);

	let appearanceOpen = $state(true);
	let securityOpen = $state(false);

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

	function activeFontId(): string {
		return FONT_PRESETS.find(f => f.value === settings.fontFamily)?.id ?? 'custom';
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={onBackdropClick}>
	<div class="modal" role="dialog" aria-label="Settings" aria-modal="true">

		<div class="modal-header">
			<span class="modal-title">Settings</span>
			<button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
		</div>

		<div class="modal-body">

			<!-- ── General ───────────────────────────────────────────── -->
			<section>
				<h3 class="section-title">General</h3>
				<div class="section-content">
					<div class="setting-row">
						<span class="setting-label">Vault name</span>
						<input
							class="text-input"
							type="text"
							bind:value={settings.vaultName}
							oninput={onChange}
							placeholder="Notes"
							maxlength="32"
						/>
					</div>
					<div class="setting-row">
						<span class="setting-label">
							Home page
							<span class="setting-value">Ctrl+Shift+H</span>
						</span>
						<input
							class="text-input"
							type="text"
							bind:value={settings.homePage}
							oninput={onChange}
							placeholder="note name"
							autocomplete="off"
						/>
					</div>
					<div class="setting-row">
						<span class="setting-label">Date format</span>
						<select
							class="select-input"
							bind:value={settings.dateFormat}
							onchange={onChange}
						>
							{#each DATE_FORMATS as f}
								<option value={f.id}>{f.label} — {f.example}</option>
							{/each}
						</select>
					</div>
					<p class="section-desc">
						Shortcuts: <kbd>Ctrl+Shift+H</kbd> home · <kbd>Ctrl+Shift+P</kbd> back · <kbd>Ctrl+Shift+N</kbd> forward
					</p>
				</div>
			</section>

			<!-- ── Appearance & Custom CSS ──────────────────────────── -->
			<section>
				<button class="section-toggle" onclick={() => (appearanceOpen = !appearanceOpen)}>
					<span>Appearance</span>
					<svg class="chevron" class:open={appearanceOpen} viewBox="0 0 10 6" width="10" height="6" aria-hidden="true">
						<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
				{#if appearanceOpen}
				<div class="section-content">
					<div class="setting-row">
						<span class="setting-label">Theme</span>
						<div class="btn-group">
							{#each THEMES as t}
								<button
									class="option-btn"
									class:active={currentTheme === t.id}
									onclick={() => onSetTheme(t.id)}
								>{t.label}</button>
							{/each}
						</div>
					</div>

					<div class="setting-row">
						<span class="setting-label">Font</span>
						<div class="btn-group font-btns">
							{#each FONT_PRESETS as f}
								<button
									class="option-btn"
									class:active={activeFontId() === f.id}
									style="font-family: {f.value};"
									onclick={() => { settings.fontFamily = f.value; onChange(); }}
								>{f.label}</button>
							{/each}
						</div>
					</div>

					<div class="setting-row">
						<span class="setting-label">
							Font size
							<span class="setting-value">{settings.fontSize.toFixed(2)}rem</span>
						</span>
						<div class="slider-wrap">
							<span class="slider-bound">A</span>
							<input
								type="range"
								min="0.85" max="1.35" step="0.05"
								bind:value={settings.fontSize}
								oninput={onChange}
								class="slider"
							/>
							<span class="slider-bound large">A</span>
						</div>
					</div>

					<div class="setting-row">
						<span class="setting-label">
							Line height
							<span class="setting-value">{settings.lineHeight.toFixed(1)}</span>
						</span>
						<div class="slider-wrap">
							<span class="slider-bound">≡</span>
							<input
								type="range"
								min="1.3" max="2.2" step="0.1"
								bind:value={settings.lineHeight}
								oninput={onChange}
								class="slider"
							/>
							<span class="slider-bound spaced">≡</span>
						</div>
					</div>

					<div class="subsection-title">Custom CSS</div>
					<p class="section-desc">Injected into the page, applied live.</p>
					<textarea
						class="css-input"
						placeholder={`.tiptap-editor {\n  max-width: 720px;\n}`}
						bind:value={settings.customCss}
						oninput={onChange}
						spellcheck="false"
						autocomplete="off"
					></textarea>

					<div class="section-actions">
						<button class="reset-btn" onclick={onReset}>Reset to defaults</button>
					</div>
				</div>
				{/if}
			</section>

			<!-- ── Security ────────────────────────────────────────── -->
			<section>
				<button class="section-toggle" onclick={() => (securityOpen = !securityOpen)}>
					<span>Security</span>
					<svg class="chevron" class:open={securityOpen} viewBox="0 0 10 6" width="10" height="6" aria-hidden="true">
						<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
					</svg>
				</button>
				{#if securityOpen}
				<div class="section-content">
					<p class="section-desc">Keys are defined in <code>aura_notes.toml</code>. To rotate, update the file and restart the backend.</p>

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
	}

	/* ── Header ────────────────────────────────────────── */
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

	/* ── Body ──────────────────────────────────────────── */
	.modal-body {
		overflow-y: auto;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1.75rem;
		flex: 1;
	}

	section {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	/* Static title (General) */
	.section-title {
		margin: 0;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		padding-bottom: 0.5rem;
		border-bottom: 1px solid var(--border);
		margin-bottom: 1rem;
	}

	/* Collapsible toggle button */
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

	.section-toggle:hover {
		color: var(--text);
	}

	.chevron {
		color: var(--muted);
		transition: transform 150ms ease, color 80ms;
		flex-shrink: 0;
		transform: rotate(-90deg);
	}

	.chevron.open {
		transform: rotate(0deg);
	}

	.section-toggle:hover .chevron {
		color: var(--text);
	}

	/* Content inside collapsible sections */
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

	/* Sub-section label within Appearance */
	.subsection-title {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--muted);
		letter-spacing: 0.04em;
		padding-top: 0.25rem;
		border-top: 1px dashed var(--border);
	}

	/* Reset button row inside section */
	.section-actions {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.25rem;
	}

	/* ── Setting row ───────────────────────────────────── */
	.setting-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.setting-label {
		font-size: 0.88rem;
		color: var(--text);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-width: 90px;
	}

	.setting-value {
		font-size: 0.75rem;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}

	/* ── Button groups (theme / font) ──────────────────── */
	.btn-group {
		display: flex;
		gap: 0.3rem;
		flex-wrap: wrap;
		justify-content: flex-end;
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

	.option-btn:hover {
		background: var(--border);
		color: var(--text);
	}

	.option-btn.active {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg);
	}

	.font-btns .option-btn {
		font-size: 0.85rem;
	}

	/* ── Sliders ───────────────────────────────────────── */
	.slider-wrap {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
	}

	.slider-bound {
		font-size: 0.75rem;
		color: var(--muted);
		flex-shrink: 0;
		width: 14px;
		text-align: center;
		line-height: 1;
	}

	.slider-bound.large {
		font-size: 1.1rem;
	}

	.slider-bound.spaced {
		letter-spacing: 0.2em;
	}

	.slider {
		flex: 1;
		accent-color: var(--accent);
		cursor: pointer;
		height: 3px;
	}

	/* ── Custom CSS ────────────────────────────────────── */
	.css-input {
		width: 100%;
		min-height: 120px;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.6rem 0.75rem;
		font-size: 0.8rem;
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		color: var(--text);
		resize: vertical;
		outline: none;
		line-height: 1.6;
		box-sizing: border-box;
	}

	.css-input:focus {
		border-color: var(--accent);
	}

	.css-input::placeholder {
		color: var(--muted);
		opacity: 0.7;
	}

	/* ── Footer ────────────────────────────────────────── */
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

	.reset-btn {
		background: none;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.25rem 0.65rem;
		font-size: 0.78rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
	}

	.reset-btn:hover {
		border-color: #e57373;
		color: #e57373;
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

	/* ── Security ─────────────────────────────────────── */
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

	.key-copy.copied {
		background: var(--accent);
		border-color: var(--accent);
		color: var(--bg);
	}

	section code {
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.8em;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0.05em 0.3em;
	}

	.section-desc kbd {
		font-size: 0.75rem;
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--sidebar-bg);
		font-family: inherit;
		color: var(--text);
	}

	/* ── Text input ───────────────────────────────────── */
	.text-input {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.6rem;
		font-size: 0.85rem;
		font-family: inherit;
		color: var(--text);
		outline: none;
		width: 160px;
		transition: border-color 80ms;
	}

	.text-input:focus {
		border-color: var(--accent);
	}

	.select-input {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.3rem 0.6rem;
		font-size: 0.85rem;
		font-family: inherit;
		color: var(--text);
		outline: none;
		cursor: pointer;
		transition: border-color 80ms;
	}

	.select-input:focus {
		border-color: var(--accent);
	}

	/* ── Mobile ────────────────────────────────────────── */
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

		.setting-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}

		.btn-group {
			justify-content: flex-start;
		}

		.slider-wrap {
			width: 100%;
		}
	}
</style>
