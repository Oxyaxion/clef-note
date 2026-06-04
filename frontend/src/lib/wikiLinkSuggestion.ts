import { Extension } from '@tiptap/core';
import Suggestion, {
	type SuggestionProps,
	type SuggestionKeyDownProps,
} from '@tiptap/suggestion';
import { PluginKey } from '@tiptap/pm/state';
import type { Editor, Range } from '@tiptap/core';
import { escapeHtml } from './utils';
import { buildSuggestionMenu } from './suggestionMenu';
import { createSuggestionPopup, type SuggestionPopup } from './suggestionPopup';

const WikiLinkSuggestionKey = new PluginKey('wikiLinkSuggestion');

interface WikiItem {
	display: string;   // text shown in the menu
	target: string;    // canonical note name used in the link
	aliasOf?: string;  // if this is an alias, the canonical note name
}

function renderWikiItem(item: WikiItem, btn: HTMLButtonElement) {
	const icon = document.createElement('span');
	icon.className = 'slash-menu-icon';
	icon.textContent = '[[';

	const text = document.createElement('span');
	text.className = 'slash-menu-text';
	text.innerHTML = item.aliasOf
		? `<span class="slash-menu-title">${escapeHtml(item.display)}</span>` +
		  `<span class="slash-menu-desc">→ ${escapeHtml(item.aliasOf)}</span>`
		: `<span class="slash-menu-title">${escapeHtml(item.display)}</span>`;

	btn.appendChild(icon);
	btn.appendChild(text);
}

function insertWikiLink(editor: Editor, range: Range, target: string) {
	editor
		.chain()
		.focus()
		.deleteRange(range)
		.insertContent({ type: 'wikiLink', attrs: { target } })
		.run();
}

/**
 * Creates the WikiLink suggestion extension.
 * @param getNotes  Returns the current note name list (called on each keystroke).
 * @param getAliases Returns the current alias → canonical note map.
 */
export function createWikiLinkSuggestion(
	getNotes: () => string[],
	getAliases: () => Record<string, string>
) {
	return Extension.create({
		name: 'wikiLinkSuggestion',

		addProseMirrorPlugins() {
			return [
				Suggestion<WikiItem>({
					editor: this.editor,
					pluginKey: WikiLinkSuggestionKey,
					char: '[[',
					allowSpaces: true,
					startOfLine: false,

					items: ({ query }): WikiItem[] => {
						const q = query.toLowerCase();
						const all = getNotes();

						// Normal notes
						const filtered = q
							? all.filter((n) => n.toLowerCase().includes(q))
							: all;
						const noteItems: WikiItem[] = filtered.slice(0, 10).map((name) => ({
							display: name,
							target: name,
						}));

						if (!q) return noteItems;

						// Alias matches (only when there's a query)
						const aliasMap = getAliases();
						const noteSet = new Set(noteItems.map((i) => i.target));
						const aliasItems: WikiItem[] = [];
						for (const [alias, canonical] of Object.entries(aliasMap)) {
							if (!alias.toLowerCase().includes(q)) continue;
							// Skip if canonical already shown as a normal note match
							if (noteSet.has(canonical)) continue;
							aliasItems.push({ display: alias, target: canonical, aliasOf: canonical });
						}

						return [...noteItems, ...aliasItems].slice(0, 12);
					},

					render: () => {
						let popup: SuggestionPopup;
						let menu: ReturnType<typeof buildSuggestionMenu<WikiItem>>;
						let editorRef: Editor;
						let rangeRef: Range;

						const doInsert = (item: WikiItem) => {
							insertWikiLink(editorRef, rangeRef, item.target);
						};

						return {
							onStart(props: SuggestionProps<WikiItem>) {
								editorRef = props.editor;
								rangeRef = props.range;
								menu = buildSuggestionMenu(renderWikiItem, doInsert, 'No notes found');
								menu.update(props.items);

								popup = createSuggestionPopup(
									menu.el,
									() => props.clientRect?.() ?? new DOMRect(0, 0, 0, 0),
								);
							},

							onUpdate(props: SuggestionProps<WikiItem>) {
								editorRef = props.editor;
								rangeRef = props.range;
								menu.update(props.items);
								popup?.setRect(() => props.clientRect?.() ?? new DOMRect(0, 0, 0, 0));
							},

							onKeyDown({ event }: SuggestionKeyDownProps): boolean {
								if (event.key === 'ArrowDown') { menu.move(1); return true; }
								if (event.key === 'ArrowUp')   { menu.move(-1); return true; }
								if (event.key === 'Enter' || event.key === 'Tab') {
									const item = menu.select();
									if (item) doInsert(item);
									return true;
								}
								if (event.key === 'Escape') { popup?.hide(); return true; }
								return false;
							},

							onExit() {
								popup?.destroy();
							},
						};
					},

					command: ({ editor, range, props }) => {
						insertWikiLink(editor, range, props.target);
					},
				}),
			];
		},
	});
}
