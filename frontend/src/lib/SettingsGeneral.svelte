<script lang="ts">
	import { DATE_FORMATS, type AppSettings } from './settings';

	interface Props {
		settings: AppSettings;
		onChange: () => void;
	}

	let { settings = $bindable(), onChange }: Props = $props();
</script>

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

	@media (max-width: 640px) {
		.setting-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 0.5rem;
		}
	}
</style>
