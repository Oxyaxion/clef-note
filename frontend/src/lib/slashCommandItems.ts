import { EMOJI_MAP } from './emojiShortcodes';
import { emit } from './events';
import { getDateFormat } from './dateFormat.svelte';
import type { Editor, Range } from '@tiptap/core';

export interface CommandItem {
	title: string;
	description: string;
	icon: string;
	keywords?: string[];
	onlyWhenFiltered?: boolean;
	command: (opts: { editor: Editor; range: Range }) => void;
}

function formatDate(): string {
	const d = new Date();
	switch (getDateFormat()) {
		case 'eu':  return d.toLocaleDateString('fr-FR');
		case 'iso': return d.toISOString().slice(0, 10);
		case 'us':  return d.toLocaleDateString('en-US');
		default:    return d.toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
	}
}

function formatTimestamp(): string {
	const d = new Date();
	switch (getDateFormat()) {
		case 'eu':  return `${d.toLocaleDateString('fr-FR')} ${d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}`;
		case 'iso': return d.toISOString().slice(0, 16).replace('T', ' ');
		case 'us':  return d.toLocaleString('en-US');
		default:    return d.toLocaleString('en-US');
	}
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
