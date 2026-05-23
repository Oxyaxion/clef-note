import { Extension } from '@tiptap/core';
import { EMOJI_MAP } from './emojiShortcodes';
import { emit } from './events';
import Suggestion, {
	type SuggestionProps,
	type SuggestionKeyDownProps,
} from '@tiptap/suggestion';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import tippy, { type Instance as TippyInstance } from 'tippy.js';
import type { Editor, Range } from '@tiptap/core';

const SlashCommandKey = new PluginKey('slashCommand');
const SlashInputTrackerKey = new PluginKey('slashInputTracker');

// True only when the last transaction was an actual edit (character typed).
// Prevents the slash-command menu from opening when the cursor is merely
// moved to an existing '/' via mouse click or arrow keys.
let docJustChanged = false;

let _dateFormat = 'long-en';
export function setDateFormat(fmt: string) { _dateFormat = fmt; }

function formatDate(): string {
	const d = new Date();
	switch (_dateFormat) {
		case 'eu':  return d.toLocaleDateString('fr-FR');
		case 'iso': return d.toISOString().slice(0, 10);
		case 'us':  return d.toLocaleDateString('en-US');
		default:    return d.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
	}
}

function formatTimestamp(): string {
	const d = new Date();
	switch (_dateFormat) {
		case 'eu':  return `${d.toLocaleDateString('fr-FR')} ${d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}`;
		case 'iso': return d.toISOString().slice(0, 16).replace('T', ' ');
		case 'us':  return d.toLocaleString('en-US');
		default:    return d.toLocaleString('en-US');
	}
}

export interface CommandItem {
	title: string;
	description: string;
	icon: string;
	keywords?: string[];
	onlyWhenFiltered?: boolean;
	command: (opts: { editor: Editor; range: Range }) => void;
}

export const ALL_ITEMS: CommandItem[] = [
	// — Text blocks —
	{
		title: 'Heading 1',
		description: 'Large section heading',
		icon: 'H1',
		keywords: ['h1', 'title', 'heading'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setNode('heading', { level: 1 }).run(),
	},
	{
		title: 'Heading 2',
		description: 'Medium section heading',
		icon: 'H2',
		keywords: ['h2', 'title', 'heading'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setNode('heading', { level: 2 }).run(),
	},
	{
		title: 'Heading 3',
		description: 'Small heading',
		icon: 'H3',
		keywords: ['h3', 'title', 'heading'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setNode('heading', { level: 3 }).run(),
	},
	{
		title: 'Paragraph',
		description: 'Normal text',
		icon: '¶',
		keywords: ['paragraph', 'text', 'normal', 'p'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setParagraph().run(),
	},

	// — Lists —
	{
		title: 'Bullet list',
		description: 'Unordered list',
		icon: '•—',
		keywords: ['bullet', 'list', 'ul', 'unordered'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleBulletList().run(),
	},
	{
		title: 'Numbered list',
		description: 'Ordered list',
		icon: '1.',
		keywords: ['ordered', 'list', 'ol', 'numbered'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleOrderedList().run(),
	},
	{
		title: 'Checklist',
		description: 'Task list with checkboxes',
		icon: '☐',
		keywords: ['todo', 'task', 'check', 'checkbox'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleTaskList().run(),
	},

	// — Special blocks —
	{
		title: 'Inline code',
		description: 'Monospace code snippet',
		icon: '``',
		keywords: ['code', 'inline', 'monospace', 'backtick'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleCode().run(),
	},
	{
		title: 'Code block',
		description: 'Code with syntax highlighting',
		icon: '</>',
		keywords: ['code', 'pre', 'monospace', 'snippet', 'block'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setCodeBlock({ language: '' }).run(),
	},
	// ── Language-specific code blocks (visible only when filtering) ──
	...(([
		['TypeScript', 'typescript', 'TS',  ['ts', 'tsx', 'javascript', 'js']],
		['JavaScript', 'javascript', 'JS',  ['js', 'jsx', 'ts', 'node']],
		['Python',     'python',     'Py',  ['py', 'python3', 'script']],
		['Rust',       'rust',       'Rs',  ['rs', 'cargo']],
		['Go',         'go',         'Go',  ['golang']],
		['Bash',       'bash',       'sh',  ['shell', 'sh', 'zsh', 'terminal', 'script']],
		['JSON',       'json',       '{}',  ['json', 'data', 'config']],
		['SQL',        'sql',        'SQL', ['query', 'database', 'db', 'mysql', 'postgres']],
		['HTML',       'html',       'HTML',['markup', 'web']],
		['CSS',        'css',        'CSS', ['style', 'stylesheet']],
		['Markdown',   'markdown',   'Md',  ['md', 'text']],
		['YAML',       'yaml',       'YML', ['yml', 'config', 'docker']],
		['C / C++',    'cpp',        'C++', ['c', 'cpp', 'c++', 'header']],
		['Java',       'java',       'Java',['jvm', 'class']],
	] as const).map(([title, lang, icon, extra]) => ({
		title: `Code — ${title}`,
		description: `${title} code block`,
		icon,
		keywords: ['code', 'block', lang, ...(extra as readonly string[])],
		onlyWhenFiltered: true,
		command: ({ editor, range }: { editor: Editor; range: Range }) =>
			editor.chain().focus().deleteRange(range).setCodeBlock({ language: lang }).run(),
	}))),
	{
		title: 'Quote',
		description: 'Indented block quote',
		icon: '"',
		keywords: ['quote', 'blockquote'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleBlockquote().run(),
	},
	{
		title: 'Table',
		description: 'Table with headers',
		icon: '⊞',
		keywords: ['table', 'grid'],
		command: ({ editor, range }) =>
			editor
				.chain()
				.focus()
				.deleteRange(range)
				.insertTable({ rows: 3, cols: 3, withHeaderRow: true })
				.run(),
	},
	{
		title: 'Divider',
		description: 'Horizontal rule',
		icon: '—',
		keywords: ['hr', 'divider', 'rule', 'separator'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setHorizontalRule().run(),
	},

	// — Links —
	{
		title: 'Link',
		description: 'Insert an external URL',
		icon: '🔗',
		keywords: ['link', 'url', 'href', 'http', 'web', 'externe'],
		command: ({ editor, range }) => {
			editor.chain().focus().deleteRange(range).run();
			const { from } = editor.state.selection;
			const coords = editor.view.coordsAtPos(from);
			emit(editor.view.dom, 'link-prompt',
				{ x: coords.left, y: coords.bottom + 8, currentUrl: '', selectedText: '' },
				{ bubbles: true }
			);
		},
	},

	// — Zettelkasten —
	{
		title: 'Wiki link',
		description: 'Type [[name]] to create a link',
		icon: '[[',
		keywords: ['wiki', 'link', 'note', 'backlink', 'zettelkasten'],
		command: ({ editor, range }) => {
			editor.chain().focus().deleteRange(range).insertContent('[[').run();
		},
	},

	// — Dynamic queries —
	{
		title: 'Dynamic query',
		description: 'Filter notes by tags, status, date',
		icon: '{}',
		keywords: ['query', 'filter', 'tag', 'search', 'dynamic', '?'],
		command: ({ editor, range }) =>
			editor
				.chain()
				.focus()
				.deleteRange(range)
				.insertContent({ type: 'queryBlock', attrs: { query: '' } })
				.run(),
	},

	{
		title: 'Drawing',
		description: 'Insert an Excalidraw diagram',
		icon: '✎',
		keywords: ['draw', 'diagram', 'sketch', 'excalidraw', 'figure'],
		command: ({ editor, range }) => {
			const name = `drawing-${Date.now()}`;
			editor.chain().focus().deleteRange(range).insertContent({ type: 'drawingBlock', attrs: { name } }).run();
		},
	},

	// — Emojis (visible only when filtering) —
	...Object.entries(EMOJI_MAP).map(([code, emoji]) => ({
		title: `${emoji}  ${code}`,
		description: `Insert ${emoji}`,
		icon: emoji,
		keywords: ['emoji', 'icon', 'ico', code],
		onlyWhenFiltered: true as const,
		command: ({ editor, range }: { editor: Editor; range: Range }) =>
			editor.chain().focus().deleteRange(range).insertContent(emoji).run(),
	})),

	// — Useful insertions —
	{
		title: "Today's date",
		description: "Insert today's date",
		icon: '📅',
		keywords: ['date', 'today'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).insertContent(formatDate()).run(),
	},
	{
		title: 'Timestamp',
		description: 'Current date and time',
		icon: '🕐',
		keywords: ['time', 'timestamp', 'datetime'],
		command: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).insertContent(formatTimestamp()).run(),
	},
];

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

// —— Frontmatter commands (built at factory time, close over callbacks) ——

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
