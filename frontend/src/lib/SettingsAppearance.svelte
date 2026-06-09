<script lang="ts">
	import { THEMES, type ThemeId } from './theme';
	import { FONT_PRESETS, type AppSettings } from './settings';

	interface Props {
		settings: AppSettings;
		currentTheme: ThemeId;
		onSetTheme: (id: ThemeId) => void;
		onChange: () => void;
		onReset: () => void;
	}

	let { settings = $bindable(), currentTheme, onSetTheme, onChange, onReset }: Props = $props();

	let open = $state(true);

	function activeFontId(): string {
		return FONT_PRESETS.find(f => f.value === settings.fontFamily)?.id ?? 'custom';
	}
</script>

<section>
	<button class="section-toggle" onclick={() => (open = !open)}>
		<span>Appearance</span>
		<svg class="chevron" class:open viewBox="0 0 10 6" width="10" height="6" aria-hidden="true">
			<path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>
	{#if open}
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

	.subsection-title {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--muted);
		letter-spacing: 0.04em;
		padding-top: 0.25rem;
		border-top: 1px dashed var(--border);
	}

	.section-actions {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.25rem;
	}

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

	.btn-group {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		border: 1px solid var(--border);
		border-radius: 6px;
		overflow: hidden;
		background: var(--sidebar-bg);
	}

	.option-btn {
		background: transparent;
		border: none;
		border-right: 1px solid var(--border);
		padding: 0.25rem 0.75rem;
		font-size: 0.78rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
		transition: background 80ms, color 80ms;
		white-space: nowrap;
		line-height: 1.5;
	}

	.option-btn:last-child { border-right: none; }

	.option-btn:hover { background: var(--border); color: var(--text); }

	.option-btn.active {
		background: var(--accent);
		color: var(--bg);
	}

	.font-btns .option-btn { font-size: 0.78rem; }

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

	.slider-bound.large { font-size: 1.1rem; }

	.slider-bound.spaced { letter-spacing: 0.2em; }

	.slider {
		flex: 1;
		accent-color: var(--accent);
		cursor: pointer;
		height: 3px;
	}

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

	.css-input:focus { border-color: var(--accent); }

	.css-input::placeholder { color: var(--muted); opacity: 0.7; }

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

	.reset-btn:hover { border-color: #e57373; color: #e57373; }

	@media (max-width: 640px) {
		.setting-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}

		.btn-group { justify-content: flex-start; }

		.slider-wrap { width: 100%; }
	}
</style>
