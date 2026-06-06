<script lang="ts">
	interface Props {
		message: string;
		confirmLabel?: string;
		onConfirm: () => void;
		onCancel: () => void;
	}

	let { message, confirmLabel = 'Confirm', onConfirm, onCancel }: Props = $props();

	function onBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onCancel();
	}

	function onDialogKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') { e.preventDefault(); onConfirm(); }
		if (e.key === 'Escape') onCancel();
	}

	function focusTrap(el: HTMLElement) {
		el.focus();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="backdrop"
	role="presentation"
	onclick={(e) => { if (e.target === e.currentTarget) onCancel(); }}
	onkeydown={onBackdropKeydown}
>
	<dialog
		open
		class="dialog"
		role="alertdialog"
		aria-modal="true"
		aria-describedby="dlg-msg"
		use:focusTrap
		onkeydown={onDialogKeydown}
	>
		<p id="dlg-msg">{message}</p>
		<div class="actions">
			<button class="btn-cancel" onclick={onCancel}>Cancel</button>
			<button class="btn-confirm" onclick={onConfirm}>{confirmLabel}</button>
		</div>
	</dialog>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		backdrop-filter: blur(2px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 500;
	}

	.dialog {
		position: relative; /* override browser UA absolute positioning so flexbox centers it */
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 10px;
		box-shadow: 0 16px 48px rgba(0, 0, 0, 0.25);
		padding: 1.5rem;
		max-width: 380px;
		width: calc(100vw - 2rem);
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		outline: none;
	}

	p {
		margin: 0;
		font-size: 0.95rem;
		line-height: 1.5;
		color: var(--text);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.btn-cancel,
	.btn-confirm {
		padding: 0.35rem 0.9rem;
		border-radius: 6px;
		font-size: 0.88rem;
		font-family: inherit;
		cursor: pointer;
		border: 1px solid var(--border);
	}

	.btn-cancel {
		background: var(--sidebar-bg);
		color: var(--text);
	}

	.btn-cancel:hover { background: var(--border); }

	.btn-confirm {
		background: var(--color-danger);
		border-color: var(--color-danger);
		color: #fff;
	}

	.btn-confirm:hover { opacity: 0.88; }
</style>
