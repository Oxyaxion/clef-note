import Image from '@tiptap/extension-image';

export const ResizableImage = Image.extend({
	addAttributes() {
		return {
			...this.parent?.(),
			width: {
				default: null,
				parseHTML: (el) => {
					const w = (el as HTMLElement).getAttribute('data-width')
					       ?? (el as HTMLElement).getAttribute('width');
					return w ? parseInt(w, 10) || null : null;
				},
				renderHTML: (attrs) => attrs.width ? { 'data-width': String(attrs.width) } : {},
			},
		};
	},

	addNodeView() {
		return ({ node, getPos, editor: ed }) => {
			let currentNode = node;

			const wrapper = document.createElement('span');
			wrapper.className = 'image-wrapper';
			wrapper.setAttribute('contenteditable', 'false');

			const img = document.createElement('img');
			img.src = node.attrs.src ?? '';
			img.alt = node.attrs.alt ?? '';
			if (node.attrs.width) img.style.width = `${node.attrs.width}px`;

			const handle = document.createElement('div');
			handle.className = 'image-resize-handle';
			handle.title = 'Drag to resize';

			handle.addEventListener('mousedown', (e) => {
				e.preventDefault();
				e.stopPropagation();
				const startX = e.clientX;
				const startWidth = img.getBoundingClientRect().width;

				const onMove = (ev: MouseEvent) => {
					const newW = Math.max(40, startWidth + ev.clientX - startX);
					img.style.width = `${newW}px`;
				};
				const onUp = (ev: MouseEvent) => {
					document.removeEventListener('mousemove', onMove);
					document.removeEventListener('mouseup', onUp);
					const newW = Math.max(40, Math.round(startWidth + ev.clientX - startX));
					const pos = getPos();
					if (typeof pos === 'number') {
						ed.view.dispatch(
							ed.view.state.tr.setNodeMarkup(pos, undefined, {
								...currentNode.attrs,
								width: newW,
							})
						);
					}
				};
				document.addEventListener('mousemove', onMove);
				document.addEventListener('mouseup', onUp);
			});

			wrapper.append(img, handle);

			return {
				dom: wrapper,
				update(updatedNode) {
					if (updatedNode.type.name !== 'image') return false;
					currentNode = updatedNode;
					img.src = updatedNode.attrs.src ?? '';
					img.alt = updatedNode.attrs.alt ?? '';
					img.style.width = updatedNode.attrs.width ? `${updatedNode.attrs.width}px` : '';
					return true;
				},
				destroy() {},
			};
		};
	},

	addStorage() {
		return {
			markdown: {
				serialize(state: any, node: any) {
					const alt = (node.attrs.alt ?? '').replace(/[\[\]]/g, '\\$&');
					const src = node.attrs.src ?? '';
					const width = node.attrs.width;
					if (width) {
						state.write(`![${alt}](${src} "w:${width}")`);
					} else if (node.attrs.title) {
						state.write(`![${alt}](${src} "${node.attrs.title}")`);
					} else {
						state.write(`![${alt}](${src})`);
					}
				},
				parse: {
					// Convert title="w:NNN" → data-width="NNN" before TipTap parses the DOM
					updateDOM(element: Element) {
						element.querySelectorAll('img[title]').forEach((el) => {
							const t = el.getAttribute('title') ?? '';
							if (t.startsWith('w:')) {
								const w = parseInt(t.slice(2), 10);
								if (!isNaN(w)) {
									el.setAttribute('data-width', String(w));
									el.removeAttribute('title');
								}
							}
						});
					},
				},
			},
		};
	},
});
