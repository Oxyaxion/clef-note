import { Extension } from '@tiptap/core';

/** Mod+Enter exits a blockquote by inserting a paragraph immediately after it. */
export const ExitBlockquote = Extension.create({
	name: 'exitBlockquote',
	priority: 1001,
	addKeyboardShortcuts() {
		return {
			'Mod-Enter': () => {
				const sel = this.editor.state.selection;
				const from = sel.$from;

				let bqDepth = -1;
				for (let d = from.depth; d > 0; d--) {
					if (from.node(d).type.name === 'blockquote') { bqDepth = d; break; }
				}
				if (bqDepth === -1) return false;

				return this.editor.chain()
					.insertContentAt(from.after(bqDepth), { type: 'paragraph' })
					.run();
			},
		};
	},
});

/** Enter on an empty list item lifts it out of the list instead of adding a blank item. */
export const ExitEmptyListItem = Extension.create({
	name: 'exitEmptyListItem',
	priority: 1001,
	addKeyboardShortcuts() {
		return {
			Enter: () => {
				const { selection } = this.editor.state;
				if (!selection.empty) return false;
				const anchor = selection.$anchor;
				for (let d = anchor.depth; d > 0; d--) {
					const node = anchor.node(d);
					if (node.type.name === 'listItem' || node.type.name === 'taskItem') {
						// firstChild is the paragraph/block inside the listItem;
						// childCount === 0 means truly empty (no text, no atoms)
						if ((node.firstChild?.childCount ?? 0) === 0) {
							return this.editor.commands.liftListItem(node.type);
						}
						return false;
					}
				}
				return false;
			},
		};
	},
});
