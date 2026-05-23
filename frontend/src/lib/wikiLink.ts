import { Node, mergeAttributes, InputRule } from '@tiptap/core';
import { emit } from './events';

const WIKI_LINK_RE = /\[\[([^\]]+)\]\]$/;

function replaceWikiLinksInDOM(root: Element) {
	const regex = /\[\[([^\]]+)\]\]/g;
	const textNodes: Text[] = [];

	const TEXT_NODE = 3;
	const ELEMENT_NODE = 1;
	const collect = (node: ChildNode) => {
		if (node.nodeType === TEXT_NODE) {
			textNodes.push(node as Text);
		} else if (node.nodeType === ELEMENT_NODE) {
			const tag = (node as Element).tagName;
			if (tag === 'CODE' || tag === 'PRE') return;
			node.childNodes.forEach(collect);
		}
	};
	root.childNodes.forEach(collect);

	for (const textNode of textNodes) {
		const text = textNode.textContent ?? '';
		regex.lastIndex = 0;
		if (!regex.test(text)) continue;
		regex.lastIndex = 0;

		const frag = document.createDocumentFragment();
		let last = 0;
		let m: RegExpExecArray | null;
		while ((m = regex.exec(text)) !== null) {
			if (m.index > last) frag.appendChild(document.createTextNode(text.slice(last, m.index)));
			const span = document.createElement('span');
			span.setAttribute('data-wiki-link', m[1]);
			span.textContent = `[[${m[1]}]]`;
			frag.appendChild(span);
			last = m.index + m[0].length;
		}
		if (last < text.length) frag.appendChild(document.createTextNode(text.slice(last)));
		textNode.parentNode?.replaceChild(frag, textNode);
	}
}

export const WikiLink = Node.create({
	name: 'wikiLink',
	group: 'inline',
	inline: true,
	atom: true,
	selectable: true,

	addAttributes() {
		return {
			target: {
				default: '',
				parseHTML: (el) => el.getAttribute('data-wiki-link') ?? '',
				renderHTML: (attrs) => ({ 'data-wiki-link': attrs.target }),
			},
		};
	},

	parseHTML() {
		return [{ tag: 'span[data-wiki-link]' }];
	},

	renderHTML({ node, HTMLAttributes }) {
		return [
			'span',
			mergeAttributes(HTMLAttributes, { class: 'wiki-link' }),
			`[[${node.attrs.target}]]`,
		];
	},

	addNodeView() {
		return ({ node }) => {
			const dom = document.createElement('span');
			dom.className = 'wiki-link';
			dom.setAttribute('data-wiki-link', node.attrs.target);
			dom.textContent = `[[${node.attrs.target}]]`;
			// preventDefault on mousedown stops ProseMirror from starting a click-to-select
			// sequence on this atom — without it, PM tries to apply a NodeSelection after
			// the document has already been replaced by navigation (RangeError).
			dom.addEventListener('mousedown', (e) => { e.preventDefault(); });
			dom.addEventListener('click', (e) => {
				e.preventDefault();
				e.stopPropagation();
				emit(document, 'wiki-navigate', node.attrs.target);
			});
			return { dom };
		};
	},

	addInputRules() {
		const nodeType = this.type;
		return [
			new InputRule({
				find: WIKI_LINK_RE,
				handler({ state, range, match }) {
					const target = match[1]?.trim();
					if (!target) return;
					state.tr.replaceWith(range.from, range.to, nodeType.create({ target }));
				},
			}),
		];
	},

	addStorage() {
		return {
			markdown: {
				serialize(state: any, node: any) {
					state.write(`[[${node.attrs.target}]]`);
				},
				parse: {
					updateDOM(element: Element) {
						replaceWikiLinksInDOM(element);
					},
				},
			},
		};
	},
});
