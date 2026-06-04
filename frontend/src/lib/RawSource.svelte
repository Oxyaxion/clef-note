<script lang="ts">
	interface Props {
		/** Full markdown source (frontmatter + body) to show. */
		value: string;
		/** When true the note is read-only — disable editing but still allow copy. */
		isLocked?: boolean;
		/** Called (debounced upstream) when the source text changes. */
		onInput: (text: string) => void;
	}

	let { value, isLocked = false, onInput }: Props = $props();

	// Local draft, seeded once when the view opens. We don't re-derive from
	// `value` on every keystroke: the parent persists to disk but doesn't push
	// the split state back until the view closes, so `value` stays stable here.
	let draft = $state(value);
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;

	function onTextInput(e: Event) {
		draft = (e.target as HTMLTextAreaElement).value;
		onInput(draft);
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(draft);
			copied = true;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => { copied = false; }, 1500);
		} catch {
			/* clipboard denied — ignore */
		}
	}
</script>

<div class="raw-source">
	<div class="raw-toolbar">
		<span class="raw-label">Markdown source{isLocked ? ' · read-only' : ''}</span>
		<button class="raw-copy" onclick={copy} aria-label="Copy markdown">
			{copied ? '✓ Copied' : 'Copy'}
		</button>
	</div>
	<textarea
		class="raw-textarea"
		value={draft}
		oninput={onTextInput}
		readonly={isLocked}
		spellcheck="false"
		autocapitalize="off"
		aria-label="Markdown source"
	></textarea>
</div>

<style>
	.raw-source {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		min-width: 0;
	}

	.raw-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.4rem 1rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.raw-label {
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--muted);
	}

	.raw-copy {
		background: var(--sidebar-bg);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.2rem 0.6rem;
		font-size: 0.75rem;
		color: var(--muted);
		cursor: pointer;
		font-family: inherit;
		transition: background 80ms, color 80ms;
	}

	.raw-copy:hover { background: var(--border); color: var(--text); }

	.raw-textarea {
		flex: 1;
		min-height: 0;
		width: 100%;
		resize: none;
		border: none;
		outline: none;
		background: var(--bg);
		color: var(--text);
		padding: 1rem 1.25rem;
		font-family: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
		font-size: 0.85rem;
		line-height: 1.6;
		tab-size: 2;
		white-space: pre;
		overflow: auto;
	}

	.raw-textarea[readonly] {
		color: var(--muted);
		cursor: default;
	}
</style>
