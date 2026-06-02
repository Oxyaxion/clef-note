<script lang="ts">
	import { onMount } from 'svelte';
	import { login, getAuthConfig } from './api';

	interface Props {
		onLogin: () => void;
		oidcError?: string | null;
	}
	let { onLogin, oidcError = null }: Props = $props();

	let password = $state('');
	let error = $state('');
	let loading = $state(false);
	let oidcEnabled = $state(false);
	let providerName = $state('SSO');
	let configLoaded = $state(false);

	onMount(async () => {
		const cfg = await getAuthConfig();
		oidcEnabled = cfg.oidc_enabled;
		providerName = cfg.provider_name ?? 'SSO';
		configLoaded = true;
	});

	async function submit(e: Event) {
		e.preventDefault();
		loading = true;
		error = '';
		try {
			await login(password);
			onLogin();
		} catch {
			error = 'Wrong password';
			password = '';
		} finally {
			loading = false;
		}
	}
</script>

<div class="login-wrap">
	<div class="login-card">
		<h1 class="app-name">Clef Note</h1>

		{#if !configLoaded}
			<p class="hint">Loading…</p>
		{:else if oidcEnabled}
			{#if oidcError === 'forbidden'}
				<p class="error">Access denied. Your account is not authorised.</p>
			{:else if oidcError}
				<p class="error">Authentication failed. Please try again.</p>
			{/if}
			<a href="/auth/oidc/login" class="oidc-btn">
				Connect with {providerName}
			</a>
		{:else}
			<form onsubmit={submit} style="display:contents">
				<input
					type="password"
					placeholder="Password"
					bind:value={password}
					autocomplete="current-password"
					disabled={loading}
					class="pwd-input"
				/>
				{#if error}
					<p class="error">{error}</p>
				{/if}
				<button type="submit" class="submit-btn" disabled={loading || !password}>
					{loading ? 'Signing in…' : 'Sign in'}
				</button>
			</form>
		{/if}
	</div>
</div>

<style>
	.login-wrap {
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg);
	}

	.login-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		width: 320px;
		padding: 2rem;
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 12px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
	}

	.app-name {
		font-size: 1.1rem;
		font-weight: 650;
		color: var(--text);
		margin: 0 0 0.5rem;
		letter-spacing: -0.02em;
	}

	.hint {
		font-size: 0.85rem;
		color: var(--text-muted);
		margin: 0;
	}

	.pwd-input {
		width: 100%;
		padding: 0.55rem 0.75rem;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 7px;
		font-size: 0.92rem;
		font-family: inherit;
		color: var(--text);
		outline: none;
		box-sizing: border-box;
		transition: border-color 80ms;
	}
	.pwd-input:focus { border-color: var(--accent); }

	.error {
		font-size: 0.82rem;
		color: #e57373;
		margin: 0;
	}

	.submit-btn {
		padding: 0.55rem;
		background: var(--accent);
		border: none;
		border-radius: 7px;
		color: var(--bg);
		font-size: 0.9rem;
		font-family: inherit;
		font-weight: 500;
		cursor: pointer;
		transition: opacity 80ms;
	}
	.submit-btn:hover:not(:disabled) { opacity: 0.88; }
	.submit-btn:disabled { opacity: 0.45; cursor: default; }

	.oidc-btn {
		display: block;
		padding: 0.55rem;
		background: var(--accent);
		border: none;
		border-radius: 7px;
		color: var(--bg);
		font-size: 0.9rem;
		font-family: inherit;
		font-weight: 500;
		cursor: pointer;
		text-align: center;
		text-decoration: none;
		transition: opacity 80ms;
	}
	.oidc-btn:hover { opacity: 0.88; }
</style>
