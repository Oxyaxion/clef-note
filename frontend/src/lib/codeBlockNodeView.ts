import type { Editor } from '@tiptap/core';
import type { Node as PmNode } from '@tiptap/pm/model';

export function makeCodeBlockNodeView() {
	return ({ node, getPos, editor: ed }: { node: PmNode; getPos: () => number | undefined; editor: Editor }) => {
		const dom = document.createElement('div');
		dom.className = 'code-block-wrap';

		const header = document.createElement('div');
		header.className = 'code-block-header';

		// ── Language label (click to edit) ──────────────────
		const langSpan = document.createElement('span');
		langSpan.className = 'code-lang';
		langSpan.title = 'Click to change language';
		langSpan.textContent = node.attrs.language || 'text';

		langSpan.addEventListener('mousedown', (e) => e.preventDefault());
		langSpan.addEventListener('click', () => {
			const input = document.createElement('input');
			input.value = langSpan.textContent === 'text' ? '' : (langSpan.textContent ?? '');
			input.placeholder = 'ex: typescript';
			input.className = 'code-lang-input';
			input.spellcheck = false;

			let discarding = false;

			const applyLang = () => {
				if (discarding) return;
				const newLang = input.value.trim().toLowerCase();
				const pos = typeof getPos === 'function' ? getPos() : undefined;
				if (pos !== undefined) {
					// Use setNodeMarkup by position — updateAttributes relies on
					// the current selection which may have moved when blur fires.
					ed.view.dispatch(
						ed.view.state.tr.setNodeMarkup(pos, undefined, { language: newLang || null })
					);
				}
				if (header.contains(input)) header.replaceChild(langSpan, input);
			};

			input.addEventListener('mousedown', (e) => e.stopPropagation());
			input.addEventListener('blur', applyLang);
			input.addEventListener('keydown', (e) => {
				if (e.key === 'Enter')  { e.preventDefault(); input.blur(); }
				if (e.key === 'Escape') { discarding = true; header.replaceChild(langSpan, input); }
				e.stopPropagation();
			});

			header.replaceChild(input, langSpan);
			input.focus();
			input.select();
		});

		// ── Copy button ──────────────────────────────────────
		const copyBtn = document.createElement('button');
		copyBtn.className = 'code-copy-btn';
		copyBtn.textContent = 'Copy';
		copyBtn.addEventListener('mousedown', (e) => e.preventDefault());
		copyBtn.addEventListener('click', () => {
			navigator.clipboard.writeText(codeEl.innerText ?? '');
			copyBtn.textContent = '✓ Copied';
			setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
		});

		header.appendChild(langSpan);
		header.appendChild(copyBtn);

		const pre = document.createElement('pre');
		const codeEl = document.createElement('code');
		codeEl.className = node.attrs.language ? `language-${node.attrs.language}` : '';
		pre.appendChild(codeEl);

		dom.appendChild(header);
		dom.appendChild(pre);

		return {
			dom,
			contentDOM: codeEl,
			stopEvent(event: Event) {
				return header.contains(event.target as globalThis.Node);
			},
			ignoreMutation(mutation: MutationRecord | { type: string; target: globalThis.Node }) {
				return !codeEl.contains(mutation.target);
			},
			update(updatedNode: PmNode) {
				if (updatedNode.type.name !== 'codeBlock') return false;
				const lang = updatedNode.attrs.language || '';
				if (!header.contains(document.activeElement)) {
					langSpan.textContent = lang || 'text';
				}
				codeEl.className = lang ? `language-${lang}` : '';
				return true;
			},
		};
	};
}
