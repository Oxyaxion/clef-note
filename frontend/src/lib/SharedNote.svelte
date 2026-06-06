<script lang="ts">
	import { onMount } from 'svelte';
	import Editor from './Editor.svelte';
	import { getSharedNote, type SharedNoteContent } from './api';

	interface Props {
		slug: string;
	}

	let { slug }: Props = $props();

	type ViewState =
		| { kind: 'loading' }
		| { kind: 'password' }
		| { kind: 'error'; code: 404 | 410 | number }
		| { kind: 'ready'; data: SharedNoteContent };

	let vs: ViewState = $state({ kind: 'loading' });
	let password = $state('');
	let pwError = $state('');
	let rawView = $state(false);

	async function load(pw?: string) {
		vs = { kind: 'loading' };
		const res = await getSharedNote(slug, pw);
		if (res.ok) {
			vs = { kind: 'ready', data: res.data };
		} else if (res.status === 401) {
			vs = { kind: 'password' };
			if (pw) pwError = 'Incorrect password.';
		} else {
			vs = { kind: 'error', code: res.status };
		}
	}

	onMount(() => { load(); });

	function submitPassword() {
		if (!password.trim()) return;
		pwError = '';
		load(password);
	}

	function downloadMd() {
		if (vs.kind !== 'ready') return;
		const filename = (vs.data.title || slug) + '.md';
		const content = `# ${vs.data.title}\n\n${vs.data.content}`;
		const blob = new Blob([content], { type: 'text/markdown' });
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = filename;
		a.click();
		URL.revokeObjectURL(a.href);
	}

</script>

<div class="shared-page">
	<header class="shared-header">
		<span class="shared-brand">clef-note · shared note</span>
		{#if vs.kind === 'ready'}
			<div class="header-actions">
				<button
					class="action-btn"
					class:active={rawView}
					onclick={() => (rawView = !rawView)}
					title="Toggle raw markdown"
				>Raw</button>
<button class="action-btn" onclick={downloadMd} title="Download .md file">Download .md</button>
			</div>
		{/if}
	</header>

	<main class="shared-main">
		{#if vs.kind === 'loading'}
			<div class="msg-box">Loading…</div>

		{:else if vs.kind === 'error'}
			<div class="msg-box">
				{#if vs.code === 410}
					<p class="msg-title">This share has expired.</p>
					<p class="msg-sub">The owner may have set an expiry date.</p>
				{:else if vs.code === 404}
					<p class="msg-title">Share not found.</p>
					<p class="msg-sub">This link may have been deleted or never existed.</p>
				{:else}
					<p class="msg-title">Something went wrong.</p>
					<p class="msg-sub">Error {vs.code}</p>
				{/if}
			</div>

		{:else if vs.kind === 'password'}
			<div class="password-gate">
				<svg class="lock-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
					<rect x="3" y="11" width="18" height="13" rx="2" stroke="currentColor" stroke-width="1.5"/>
					<path d="M7 11V7a5 5 0 0 1 10 0v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
				</svg>
				<p class="pw-title">This note is password-protected</p>
				<form class="pw-form" onsubmit={(e) => { e.preventDefault(); submitPassword(); }}>
					<input
						class="pw-input"
						type="password"
						bind:value={password}
						placeholder="Enter password…"
						autocomplete="current-password"
					/>
					<button class="pw-btn" type="submit">Unlock</button>
				</form>
				{#if pwError}
					<p class="pw-error">{pwError}</p>
				{/if}
			</div>

		{:else if vs.kind === 'ready'}
			<article class="note-article">
				{#if !rawView}
					<h1 class="note-title">{vs.data.title}</h1>
					<div class="editor-wrap">
						<Editor
							noteContent={vs.data.content}
							noteKey={slug}
							noteNames={[]}
							isLocked={true}
							noAuth={true}
							onEdit={() => {}}
						/>
					</div>
				{:else}
					<pre class="raw-content">{vs.data.content}</pre>
				{/if}
			</article>
		{/if}
	</main>
</div>

<style>
	.shared-page {
		min-height: 100dvh;
		display: flex;
		flex-direction: column;
		background: var(--bg, #ffffff);
		color: var(--text, #1a1a1a);
	}

	/* ── Header ── */
	.shared-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.6rem 1.5rem;
		border-bottom: 1px solid var(--border, #e5e5e5);
		background: var(--sidebar-bg, #f8f8f8);
		flex-shrink: 0;
		gap: 1rem;
	}

	.shared-brand {
		font-size: 0.78rem;
		color: var(--muted, #888);
		font-weight: 500;
		letter-spacing: 0.03em;
		white-space: nowrap;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.action-btn {
		padding: 0.25rem 0.65rem;
		border-radius: 5px;
		border: 1px solid var(--border, #e5e5e5);
		background: var(--bg, #fff);
		color: var(--text, #1a1a1a);
		font-size: 0.78rem;
		cursor: pointer;
		font-family: inherit;
		text-decoration: none;
		transition: border-color 100ms;
		line-height: 1.4;
	}

	.action-btn:hover, .action-btn.active {
		border-color: var(--accent, #5b5bd6);
		color: var(--accent, #5b5bd6);
	}

	/* ── Main ── */
	.shared-main {
		flex: 1;
		display: flex;
		justify-content: center;
		padding: 2rem 1rem 4rem;
	}

	/* ── Status messages ── */
	.msg-box {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		flex: 1;
		color: var(--muted, #888);
		font-size: 0.9rem;
	}

	.msg-title {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--text, #1a1a1a);
		margin: 0;
	}

	.msg-sub { margin: 0; color: var(--muted, #888); font-size: 0.88rem; }

	/* ── Password gate ── */
	.password-gate {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		max-width: 320px;
		width: 100%;
		margin-top: 4rem;
	}

	.lock-icon {
		width: 40px;
		height: 40px;
		color: var(--muted, #888);
	}

	.pw-title {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text, #1a1a1a);
		margin: 0;
		text-align: center;
	}

	.pw-form {
		display: flex;
		width: 100%;
		gap: 0.5rem;
	}

	.pw-input {
		flex: 1;
		padding: 0.45rem 0.75rem;
		border: 1px solid var(--border, #e5e5e5);
		border-radius: 6px;
		font-size: 0.88rem;
		font-family: inherit;
		background: var(--bg, #fff);
		color: var(--text, #1a1a1a);
		outline: none;
	}

	.pw-input:focus { border-color: var(--accent, #5b5bd6); }

	.pw-btn {
		padding: 0.45rem 1rem;
		border-radius: 6px;
		border: 1px solid var(--accent, #5b5bd6);
		background: var(--accent, #5b5bd6);
		color: #fff;
		font-size: 0.88rem;
		font-family: inherit;
		cursor: pointer;
	}

	.pw-error {
		font-size: 0.82rem;
		color: var(--color-danger, #e53e3e);
		margin: 0;
	}

	/* ── Note article ── */
	.note-article {
		width: 100%;
		max-width: 740px;
	}

	.note-title {
		font-size: 1.9rem;
		font-weight: 700;
		letter-spacing: -0.03em;
		margin: 0 0 2rem;
		color: var(--text, #1a1a1a);
		padding: 0 3rem;
	}

	/* Raw view */
	.raw-content {
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.85rem;
		line-height: 1.7;
		color: var(--text, #1a1a1a);
		white-space: pre-wrap;
		word-break: break-word;
		background: none;
		border: none;
		padding: 0 3rem;
		margin: 0;
	}

	@media (max-width: 640px) {
		.shared-header { padding: 0.6rem 1rem; }
		.note-title, .raw-content { padding: 0 1rem; }
		.header-actions { gap: 0.3rem; }
		.action-btn { padding: 0.25rem 0.5rem; }
	}
</style>
