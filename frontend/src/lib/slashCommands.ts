import { Extension } from '@tiptap/core';
import Suggestion, {
	type SuggestionProps,
	type SuggestionKeyDownProps,
} from '@tiptap/suggestion';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import tippy, { type Instance as TippyInstance } from 'tippy.js';
import type { Editor, Range } from '@tiptap/core';
import { ALL_ITEMS, type CommandItem } from './slashCommandItems';

export { setDateFormat } from './dateFormat.svelte';
export { ALL_ITEMS, type CommandItem } from './slashCommandItems';

const SlashCommandKey = new PluginKey('slashCommand');
const SlashInputTrackerKey = new PluginKey('slashInputTracker');

// True only when the last transaction was an actual edit (character typed).
// Prevents the slash-command menu from opening when the cursor is merely
// moved to an existing '/' via mouse click or arrow keys.
let docJustChanged = false;

// —— Menu DOM ——

function buildMenu(onSelect: (item: CommandItem) => void) {
	let selectedIndex = 0;
	let items: CommandItem[] = [];

	const el = document.createElement('div');
	el.className = 'slash-menu';

	function render() {
		el.innerHTML = '';
		if (items.length === 0) {
			const empty = document.createElement('div');
			empty.className = 'slash-menu-empty';
			empty.textContent = 'No commands found';
			el.appendChild(empty);
			return;
		}
		items.forEach((item, i) => {
			const btn = document.createElement('button');
			btn.className = 'slash-menu-item' + (i === selectedIndex ? ' selected' : '');

			const icon = document.createElement('span');
			icon.className = 'slash-menu-icon';
			icon.textContent = item.icon;

			const text = document.createElement('span');
			text.className = 'slash-menu-text';
			text.innerHTML =
				`<span class="slash-menu-title">${item.title}</span>` +
				`<span class="slash-menu-desc">${item.description}</span>`;

			btn.appendChild(icon);
			btn.appendChild(text);
			btn.addEventListener('mousedown', (e) => {
				e.preventDefault();
				onSelect(item);
			});
			el.appendChild(btn);
		});
	}

	return {
		el,
		update(newItems: CommandItem[]) {
			items = newItems;
			selectedIndex = 0;
			render();
		},
		move(delta: number) {
			if (items.length === 0) return;
			selectedIndex = (selectedIndex + delta + items.length) % items.length;
			render();
			el.querySelectorAll('.slash-menu-item')[selectedIndex]?.scrollIntoView({ block: 'nearest' });
		},
		select(): CommandItem | null {
			return items[selectedIndex] ?? null;
		},
	};
}

// —— Extension TipTap ——

export const SlashCommand = Extension.create({
	name: 'slashCommand',

	addOptions() {
		return {
			suggestion: {
				pluginKey: SlashCommandKey,
				char: '/',
				startOfLine: false,
				allowSpaces: false,

				items: ({ query }: { query: string }) => {
					const q = query.toLowerCase();
					if (!q) return ALL_ITEMS.filter((item) => !item.onlyWhenFiltered);
					return ALL_ITEMS.filter(
						(item) =>
							item.title.toLowerCase().includes(q) ||
							item.description.toLowerCase().includes(q) ||
							item.keywords?.some((k) => k.includes(q))
					);
				},

				render: () => {
					let popup: TippyInstance[];
					let menu: ReturnType<typeof buildMenu>;
					let editorRef: Editor;
					let rangeRef: Range;

					return {
						onStart(props: SuggestionProps<CommandItem>) {
							editorRef = props.editor;
							rangeRef = props.range;

							menu = buildMenu((item) => {
								item.command({ editor: editorRef, range: rangeRef });
							});
							menu.update(props.items);

							popup = tippy('body', {
								getReferenceClientRect: () =>
									props.clientRect?.() ?? new DOMRect(0, 0, 0, 0),
								appendTo: () => document.body,
								content: menu.el,
								showOnCreate: true,
								interactive: true,
								trigger: 'manual',
								placement: 'bottom-start',
							});
						},

						onUpdate(props: SuggestionProps<CommandItem>) {
							editorRef = props.editor;
							rangeRef = props.range;
							menu.update(props.items);
							popup?.[0]?.setProps({
								getReferenceClientRect: () =>
									props.clientRect?.() ?? new DOMRect(0, 0, 0, 0),
							});
						},

						onKeyDown({ event }: SuggestionKeyDownProps): boolean {
							if (event.key === 'ArrowDown') { menu.move(1); return true; }
							if (event.key === 'ArrowUp')   { menu.move(-1); return true; }
							if (event.key === 'Enter') {
								const item = menu.select();
								if (item) item.command({ editor: editorRef, range: rangeRef });
								return true;
							}
							if (event.key === 'Escape') { popup?.[0]?.hide(); return true; }
							return false;
						},

						onExit() {
							popup?.[0]?.destroy();
						},
					};
				},

				command: ({
					editor,
					range,
					props,
				}: {
					editor: Editor;
					range: Range;
					props: CommandItem;
				}) => {
					props.command({ editor, range });
				},
			},
		};
	},

	addProseMirrorPlugins() {
		return [
			// Registered first so docJustChanged is set before Suggestion's allow() runs.
			new Plugin({
				key: SlashInputTrackerKey,
				state: {
					init: () => false,
					apply: (tr) => {
						docJustChanged = tr.docChanged;
						return tr.docChanged;
					},
				},
			}),
			Suggestion({
				editor: this.editor,
				...this.options.suggestion,
				allow: () => docJustChanged,
			}),
		];
	},
});
