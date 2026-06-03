/**
 * Paragraph extension that preserves intentional blank lines in the markdown
 * round-trip.
 *
 * Pressing Enter in the editor creates empty paragraphs. The default markdown
 * serialization loses them: CommonMark cannot represent consecutive blank lines
 * — any run of blank lines collapses into a single paragraph break — so on the
 * next load the empty paragraphs disappear.
 *
 * To keep them, an empty paragraph is serialized as a line containing a single
 * non-breaking space (U+00A0). That line survives both serialization and
 * re-parsing (a plain space would be stripped by trailing-whitespace trimming,
 * e.g. by git or editors; the nbsp does not). The trailing empty paragraph is
 * skipped so a blank note stays truly empty on disk.
 */
import Paragraph from '@tiptap/extension-paragraph';

const NBSP = '\u00A0';

export const ParagraphMd = Paragraph.extend({
	addStorage() {
		return {
			markdown: {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				serialize(state: any, node: any, parent: any, index: number) {
					const isEmpty = node.content.size === 0;
					const isLast = !parent || index === parent.childCount - 1;
					if (isEmpty && !isLast) {
						state.write(NBSP);
						state.closeBlock(node);
						return;
					}
					state.renderInline(node);
					state.closeBlock(node);
				},
				parse: {
					// handled by markdown-it
				},
			},
		};
	},
});
