<script lang="ts">
	import { THEMES } from './theme';
	import { FONT_PRESETS, PARTITION_DEFAULTS, type AppSettings, type PartitionSettings } from './settings';

	interface Props {
		settings: AppSettings;
		activePartitionSlug?: string;
		activePartitionName?: string;
		onChange: () => void;
		onReset: () => void;
	}

	let { settings = $bindable(), activePartitionSlug = '', activePartitionName = '', onChange, onReset }: Props = $props();

	let open = $state(true);

	function ps(): PartitionSettings {
		return settings.partitions?.[activePartitionSlug] ?? {};
	}

	function setPs<K extends keyof PartitionSettings>(key: K, val: PartitionSettings[K]) {
		if (!activePartitionSlug) return;
		settings.partitions = {
			...settings.partitions,
			[activePartitionSlug]: { ...ps(), [key]: val },
		};
		onChange();
	}

	function activeFontId(): string {
		return FONT_PRESETS.find(f => f.value === (ps().fontFamily ?? PARTITION_DEFAULTS.fontFamily))?.id ?? 'custom';
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

		{#if activePartitionName}
			<div class="scope-label">Per partition · <strong>{activePartitionName}</strong></div>
		{/if}

		<div class="setting-row">
			<span class="setting-label">Theme</span>
			<div class="btn-group">
				{#each THEMES as t}
					<button
						class="option-btn"
						class:active={(ps().theme ?? PARTITION_DEFAULTS.theme) === t.id}
						onclick={() => setPs('theme', t.id)}
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
						onclick={() => setPs('fontFamily', f.value)}
					>{f.label}</button>
				{/each}
			</div>
		</div>

		<div class="setting-row">
			<span class="setting-label">
				Font size
				<span class="setting-value">{(ps().fontSize ?? PARTITION_DEFAULTS.fontSize).toFixed(2)}rem</span>
			</span>
			<div class="slider-wrap">
				<span class="slider-bound">A</span>
				<input
					type="range"
					min="0.85" max="1.35" step="0.05"
					value={ps().fontSize ?? PARTITION_DEFAULTS.fontSize}
					oninput={(e) => setPs('fontSize', parseFloat((e.target as HTMLInputElement).value))}
					class="slider"
				/>
				<span class="slider-bound large">A</span>
			</div>
		</div>

		<div class="setting-row">
			<span class="setting-label">
				Line height
				<span class="setting-value">{(ps().lineHeight ?? PARTITION_DEFAULTS.lineHeight).toFixed(1)}</span>
			</span>
			<div class="slider-wrap">
				<span class="slider-bound">≡</span>
				<input
					type="range"
					min="1.3" max="2.2" step="0.1"
					value={ps().lineHeight ?? PARTITION_DEFAULTS.lineHeight}
					oninput={(e) => setPs('lineHeight', parseFloat((e.target as HTMLInputElement).value))}
					class="slider"
				/>
				<span class="slider-bound spaced">≡</span>
			</div>
		</div>

		<div class="global-divider">Global</div>

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

	.scope-label {
		font-size: 0.72rem;
		color: var(--accent);
		opacity: 0.8;
		margin-bottom: -0.25rem;
	}

	.global-divider {
		font-size: 0.68rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		padding-top: 0.5rem;
		border-top: 1px dashed var(--border);
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
		gap: 0.25rem;
		flex-wrap: wrap;
		justify-content: flex-end;
	}

	.option-btn {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 0.18rem 0.6rem;
		font-size: 0.76rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
		transition: background 80ms, border-color 80ms, color 80ms;
		white-space: nowrap;
		line-height: 1.5;
	}

	.option-btn:hover { background: var(--border); color: var(--text); }

	.option-btn.active {
		background: var(--accent);
		border-color: var(--accent);
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
