<script lang="ts">
	import { DATE_FORMATS, type AppSettings } from './settings';

	interface Props {
		settings: AppSettings;
		activePartitionSlug?: string;
		onChange: () => void;
	}

	let { settings = $bindable(), activePartitionSlug = '', onChange }: Props = $props();

	function onHomeInput(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		if (!activePartitionSlug) return;
		settings.partitions = {
			...settings.partitions,
			[activePartitionSlug]: { ...settings.partitions?.[activePartitionSlug], homePage: val },
		};
		onChange();
	}
</script>

<section>
	<h3 class="section-title">General</h3>
	<div class="section-content">
		<div class="setting-row">
			<span class="setting-label">
				Home page
				<span class="setting-value">Ctrl+Shift+H</span>
			</span>
			<input
				class="text-input"
				type="text"
				value={settings.partitions?.[activePartitionSlug]?.homePage ?? ''}
				oninput={onHomeInput}
				placeholder="note name"
				autocomplete="off"
				disabled={!activePartitionSlug}
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
		<div class="setting-row">
			<label class="setting-label" for="mobile-readonly">Read-only on mobile</label>
			<input
				id="mobile-readonly"
				type="checkbox"
				class="toggle"
				bind:checked={settings.mobileReadOnly}
				onchange={onChange}
			/>
		</div>
		<p class="section-desc">
			Shortcuts: <kbd>Ctrl+Shift+H</kbd> home · <kbd>Ctrl+Shift+L</kbd> back · <kbd>Ctrl+Shift+N</kbd> forward · <kbd>Ctrl+Shift+M</kbd> markdown source
		</p>
	</div>
</section>

<style>
	section {
		display: flex;
		flex-direction: column;
		gap: 0;
	}

	.section-title {
		margin: 0 0 1rem;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
		padding-bottom: 0.5rem;
		border-bottom: 1px solid var(--border);
	}

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

	.section-desc kbd {
		font-size: 0.75rem;
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--sidebar-bg);
		font-family: inherit;
		color: var(--text);
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
	}

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

	.text-input:focus { border-color: var(--accent); }

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

	.select-input:focus { border-color: var(--accent); }

	.toggle {
		appearance: none;
		-webkit-appearance: none;
		width: 36px;
		height: 20px;
		border-radius: 10px;
		background: var(--border);
		cursor: pointer;
		position: relative;
		flex-shrink: 0;
		transition: background 150ms;
	}

	.toggle::after {
		content: '';
		position: absolute;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--text);
		top: 3px;
		left: 3px;
		transition: transform 150ms;
	}

	.toggle:checked {
		background: var(--accent);
	}

	.toggle:checked::after {
		transform: translateX(16px);
	}

	@media (max-width: 640px) {
		.setting-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}
	}
</style>
