import { Node, mergeAttributes } from '@tiptap/core';
import type { Editor } from '@tiptap/core';
import { NodeSelection, TextSelection } from '@tiptap/pm/state';
import type { Node as PmNode } from '@tiptap/pm/model';

interface MarkdownSerializerState {
    write(text: string): void;
    ensureNewLine(): void;
    closeBlock(node: PmNode): void;
}
import { queryNotes, listTags, getFieldValues, type NoteQueryResult, type TagCount } from './api';
import { on } from './events';
import {
    getActiveToken, parsePrint, ICON_LOCKED, ICON_UNLOCKED,
    type ActiveToken, type PrintSpec,
} from './queryBlockHelpers';
import { appendResultRows, buildTagCloud, buildTagCountCloud } from './queryBlockDom';

// ── NodeView class ─────────────────────────────────────────────────────────────

class QueryBlockNodeView {
    readonly dom: HTMLElement;

    private readonly input: HTMLInputElement;
    private readonly results: HTMLDivElement;
    private readonly countEl: HTMLSpanElement;
    private readonly lockBtn: HTMLButtonElement;

    private currentNode: PmNode;
    private currentPrint: PrintSpec | null = null;
    private acSuggestions: string[] = [];
    private acIndex = -1;
    private acToken: ActiveToken | null = null;
    private acRows: NoteQueryResult[] = [];
    private debounceTimer: ReturnType<typeof setTimeout> | null = null;
    private fetchCtrl: AbortController | null = null;
    private readonly offNotesChanged: () => void;

    constructor(
        node: PmNode,
        private readonly getPos: () => number | undefined,
        private readonly editor: Editor,
    ) {
        this.currentNode = node;

        // ── Build DOM ────────────────────────────────────────────────────────
        this.dom = document.createElement('div');
        this.dom.className = 'query-block';
        this.dom.setAttribute('contenteditable', 'false');

        const header = document.createElement('div');
        header.className = 'query-block-header';

        const icon = document.createElement('span');
        icon.className = 'query-block-icon';
        icon.textContent = '{}';

        this.input = document.createElement('input');
        this.input.className = 'query-block-input';
        this.input.placeholder = '#tag  status:active  order by date desc  print title';
        this.input.value = node.attrs.query ?? '';
        this.input.spellcheck = false;

        this.countEl = document.createElement('span');
        this.countEl.className = 'query-result-count';

        this.lockBtn = document.createElement('button');
        this.lockBtn.type = 'button';
        this.lockBtn.className = 'query-block-lock';

        header.append(icon, this.input, this.countEl, this.lockBtn);
        this.dom.appendChild(header);

        this.results = document.createElement('div');
        this.results.className = 'query-block-results';
        this.dom.appendChild(this.results);

        this.applyLocked(node.attrs.locked ?? false);
        this.setupListeners();

        this.fetchResults(node.attrs.query ?? '');
        this.offNotesChanged = on(document, 'notes:changed',
            () => this.fetchResults(this.currentNode.attrs.query ?? '')
        );
    }

    // ── NodeView interface ───────────────────────────────────────────────────

    selectNode(): void {
        setTimeout(() => this.input.focus(), 0);
    }

    update(updatedNode: PmNode): boolean {
        if (updatedNode.type.name !== 'queryBlock') return false;
        this.currentNode = updatedNode;
        this.applyLocked(updatedNode.attrs.locked ?? false);
        const newQ = updatedNode.attrs.query ?? '';
        if (this.input.value !== newQ) {
            this.input.value = newQ;
            this.fetchResults(newQ);
        }
        return true;
    }

    destroy(): void {
        if (this.debounceTimer) clearTimeout(this.debounceTimer);
        this.fetchCtrl?.abort();
        this.offNotesChanged();
    }

    // ── DOM setup ────────────────────────────────────────────────────────────

    private setupListeners(): void {
        this.dom.addEventListener('mousedown', (e) => {
            const target = e.target as HTMLElement;
            if (target === this.input) {
                e.stopPropagation();
            } else if (!target.closest('button')) {
                e.preventDefault();
                this.input.focus();
            }
        });

        this.lockBtn.addEventListener('click', () => this.toggleLock());

        this.input.addEventListener('keydown', (e) => this.handleInputKeydown(e));

        const stop = (e: Event) => e.stopPropagation();
        this.input.addEventListener('keyup', stop);
        this.input.addEventListener('keypress', stop);
        this.input.addEventListener('beforeinput', stop);

        this.input.addEventListener('input', () => {
            if (this.currentNode.attrs.locked) return;
            if (this.debounceTimer) clearTimeout(this.debounceTimer);
            this.debounceTimer = setTimeout(() => {
                this.updateQueryAttr();
                this.checkAutocompletion();
            }, 200);
        });

        this.results.addEventListener('keydown', (e) => this.handleResultsKeydown(e));
    }

    // ── Lock ─────────────────────────────────────────────────────────────────

    private applyLocked(locked: boolean): void {
        this.input.readOnly = locked;
        this.lockBtn.innerHTML = locked ? ICON_LOCKED : ICON_UNLOCKED;
        this.lockBtn.title = locked ? 'Unlock query' : 'Lock query';
        this.dom.classList.toggle('locked', locked);
    }

    private toggleLock(): void {
        const pos = this.getPos();
        if (typeof pos !== 'number') return;
        this.editor.view.dispatch(
            this.editor.view.state.tr.setNodeMarkup(pos, undefined, {
                ...this.currentNode.attrs,
                locked: !this.currentNode.attrs.locked,
            })
        );
    }

    // ── Cursor helpers ───────────────────────────────────────────────────────

    private moveCursorTo(pos: number): void {
        const { state } = this.editor.view;
        const clamped = Math.max(0, Math.min(pos, state.doc.content.size));
        this.editor.view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, clamped)));
        this.editor.view.focus();
    }

    private insertParagraphBelow(): void {
        const pos = this.getPos();
        if (typeof pos === 'number') {
            const insertAt = pos + this.currentNode.nodeSize;
            this.editor.chain()
                .insertContentAt(insertAt, { type: 'paragraph' })
                .setTextSelection(insertAt + 1)
                .focus()
                .run();
        }
    }

    private updateQueryAttr(): void {
        if (this.currentNode.attrs.locked) return;
        const pos = this.getPos();
        if (typeof pos === 'number') {
            this.editor.view.dispatch(
                this.editor.view.state.tr.setNodeMarkup(pos, undefined, {
                    ...this.currentNode.attrs,
                    query: this.input.value,
                })
            );
        }
    }

    // ── Keyboard handlers ────────────────────────────────────────────────────

    private handleInputKeydown(e: KeyboardEvent): void {
        if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            this.insertParagraphBelow();
            e.preventDefault(); e.stopPropagation(); return;
        }
        if (this.acSuggestions.length > 0) {
            if (e.key === 'ArrowDown') {
                this.acIndex = Math.min(this.acIndex + 1, this.acSuggestions.length - 1);
                this.renderBrowseMode();
                e.preventDefault(); e.stopPropagation(); return;
            }
            if (e.key === 'ArrowUp') {
                this.acIndex = Math.max(this.acIndex - 1, 0);
                this.renderBrowseMode();
                e.preventDefault(); e.stopPropagation(); return;
            }
            if (e.key === 'Tab') {
                this.applySuggestion(this.acIndex >= 0 ? this.acSuggestions[this.acIndex] : this.acSuggestions[0]);
                e.preventDefault(); e.stopPropagation(); return;
            }
            if (e.key === 'Escape') {
                this.acSuggestions = [];
                this.acIndex = -1;
                this.acToken = null;
                this.fetchResults(this.input.value);
                e.preventDefault(); e.stopPropagation(); return;
            }
        }

        const pmPos = this.getPos();
        switch (e.key) {
            case 'ArrowUp':
                if (typeof pmPos === 'number') this.moveCursorTo(pmPos);
                e.preventDefault(); e.stopPropagation(); break;
            case 'ArrowDown': {
                const firstBtn = this.results.querySelector<HTMLElement>('.query-result-name');
                if (firstBtn) {
                    firstBtn.focus();
                } else if (typeof pmPos === 'number') {
                    this.moveCursorTo(pmPos + this.currentNode.nodeSize);
                }
                e.preventDefault(); e.stopPropagation(); break;
            }
            case 'Enter':
                this.insertParagraphBelow();
                e.preventDefault(); e.stopPropagation(); break;
            case 'Escape':
                this.editor.view.focus();
                e.stopPropagation(); break;
            case 'Backspace': {
                const atStart = this.input.selectionStart === 0 && this.input.selectionEnd === 0;
                if ((this.input.value === '' || atStart) && !this.currentNode.attrs.locked && typeof pmPos === 'number') {
                    this.editor.chain()
                        .deleteRange({ from: pmPos, to: pmPos + this.currentNode.nodeSize })
                        .focus()
                        .run();
                    e.preventDefault();
                }
                e.stopPropagation(); break;
            }
            default:
                e.stopPropagation(); break;
        }
    }

    private handleResultsKeydown(e: KeyboardEvent): void {
        const items = Array.from(this.results.querySelectorAll<HTMLElement>('.query-result-name'));
        const idx = items.indexOf(document.activeElement as HTMLElement);
        if (e.key === 'ArrowDown') {
            if (idx < items.length - 1) {
                items[idx + 1].focus();
            } else {
                const pmPos = this.getPos();
                if (typeof pmPos === 'number') this.moveCursorTo(pmPos + this.currentNode.nodeSize);
            }
            e.preventDefault(); e.stopPropagation();
        } else if (e.key === 'ArrowUp') {
            if (idx > 0) items[idx - 1].focus();
            else this.input.focus();
            e.preventDefault(); e.stopPropagation();
        } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            this.insertParagraphBelow();
            e.preventDefault(); e.stopPropagation();
        } else if (e.key === 'Enter') {
            const active = document.activeElement as HTMLElement;
            if (items.includes(active)) active.click();
            e.preventDefault(); e.stopPropagation();
        } else if (e.key === 'Escape') {
            this.input.focus();
            e.preventDefault(); e.stopPropagation();
        }
    }

    // ── Autocomplete ─────────────────────────────────────────────────────────

    private stripToken(query: string, token: ActiveToken): string {
        const before = query.slice(0, token.start).replace(/\s*(AND|OR|NOT)\s*$/, '').trimEnd();
        const after = query.slice(token.end).replace(/^\s*(AND|OR|NOT)\s*/, '').trimStart();
        if (!before && !after) return '';
        if (!before) return after;
        if (!after) return before;
        return before + ' ' + after;
    }

    private renderBrowseMode(): void {
        this.results.innerHTML = '';
        this.countEl.textContent = this.acRows.length ? `${this.acRows.length}` : '';
        if (this.acSuggestions.length > 0 && this.acToken) {
            const tagCounts: TagCount[] = this.acSuggestions.map(s => ({ tag: s, count: 0 }));
            const cloud = buildTagCloud(
                tagCounts,
                (tag) => this.applySuggestion(tag),
                this.acIndex,
                this.acRows.length > 0,
                this.acToken.type === 'tag' ? '#' : '',
            );
            this.results.appendChild(cloud);
        }
        if (this.acRows.length > 0) appendResultRows(this.results, this.acRows, this.currentPrint);
    }

    private applySuggestion(value: string): void {
        if (!this.acToken) return;
        const completion = this.acToken.type === 'tag'
            ? `#${value}`
            : `${this.acToken.field}:${value}`;
        const before = this.input.value.slice(0, this.acToken.start);
        const after = this.input.value.slice(this.acToken.end);
        const sep = after.length > 0 && after[0] !== ' ' ? ' ' : '';
        this.input.value = before + completion + sep + after;
        const newPos = this.acToken.start + completion.length + sep.length;
        this.input.setSelectionRange(newPos, newPos);
        this.acSuggestions = [];
        this.acIndex = -1;
        this.acToken = null;
        this.updateQueryAttr();
        this.fetchResults(this.input.value);
    }

    private async checkAutocompletion(): Promise<void> {
        const token = getActiveToken(this.input);
        if (!token || token.partial) {
            this.acSuggestions = [];
            this.acIndex = -1;
            this.acToken = null;
            this.acRows = [];
            this.fetchResults(this.input.value);
            return;
        }
        let chipItems: string[];
        if (token.type === 'tag' || token.field === 'tag') {
            const tags = await listTags();
            chipItems = tags.map(t => t.tag);
        } else {
            chipItems = await getFieldValues(token.field);
        }
        const baseQuery = this.stripToken(this.input.value, token);
        const { cleanQuery } = parsePrint(baseQuery);
        const rows = cleanQuery ? await queryNotes(cleanQuery) : [];
        this.acSuggestions = chipItems;
        this.acIndex = -1;
        this.acToken = token;
        this.acRows = rows;
        this.renderBrowseMode();
    }

    // ── Results rendering ────────────────────────────────────────────────────

    private renderTags(tags: TagCount[]): void {
        this.results.innerHTML = '';
        this.countEl.textContent = tags.length ? `${tags.length}` : '';
        if (!tags.length) {
            this.results.innerHTML = '<span class="query-empty">No tags found</span>';
            return;
        }
        const cloud = buildTagCountCloud(tags, (tag) => {
            const newQ = `#${tag}`;
            this.input.value = newQ;
            const pos = this.getPos();
            if (typeof pos === 'number') {
                this.editor.view.dispatch(
                    this.editor.view.state.tr.setNodeMarkup(pos, undefined, {
                        ...this.currentNode.attrs,
                        query: newQ,
                    })
                );
            }
            this.fetchResults(newQ);
        });
        this.results.appendChild(cloud);
    }

    private renderResults(rows: NoteQueryResult[], print: PrintSpec | null): void {
        this.results.innerHTML = '';
        this.countEl.textContent = rows.length ? `${rows.length}` : '';
        appendResultRows(this.results, rows, print);
    }

    private async fetchResults(q: string): Promise<void> {
        this.fetchCtrl?.abort();
        const ctrl = new AbortController();
        this.fetchCtrl = ctrl;

        const { cleanQuery, print } = parsePrint(q);
        this.currentPrint = print;

        if (!cleanQuery.trim()) {
            this.countEl.textContent = '';
            this.results.innerHTML = '<span class="query-empty">Type a query above…</span>';
            this.fetchCtrl = null;
            return;
        }
        this.results.innerHTML = '<span class="query-empty">Searching…</span>';
        try {
            if (cleanQuery.trim() === '#') {
                const tags = await listTags(ctrl.signal);
                this.renderTags(tags);
            } else {
                const rows = await queryNotes(cleanQuery, ctrl.signal);
                this.renderResults(rows, print);
            }
        } catch (e) {
            if (e instanceof DOMException && e.name === 'AbortError') return;
            this.countEl.textContent = '';
            this.results.innerHTML = '<span class="query-empty query-error">Query error</span>';
        } finally {
            if (this.fetchCtrl === ctrl) this.fetchCtrl = null;
        }
    }
}

// ── Node ──────────────────────────────────────────────────────────────────────

export const QueryBlock = Node.create({
    name: 'queryBlock',
    group: 'block',
    atom: true,
    draggable: true,

    addAttributes() {
        return {
            query: { default: '' },
            locked: { default: false },
        };
    },

    parseHTML() {
        return [{
            tag: 'div[data-type="queryBlock"]',
            getAttrs: (el) => ({
                query: (el as HTMLElement).getAttribute('data-query') ?? '',
                locked: (el as HTMLElement).getAttribute('data-locked') === 'true',
            }),
        }];
    },

    renderHTML({ HTMLAttributes }) {
        return ['div', mergeAttributes(HTMLAttributes, {
            'data-type': 'queryBlock',
            'data-query': HTMLAttributes.query ?? '',
            ...(HTMLAttributes.locked ? { 'data-locked': 'true' } : {}),
            class: 'query-block-static',
        })];
    },

    addKeyboardShortcuts() {
        return {
            // Prevent accidental deletion when ProseMirror has a NodeSelection on this block.
            'Backspace': () => {
                const { selection } = this.editor.state;
                return selection instanceof NodeSelection && selection.node.type.name === 'queryBlock';
            },
            'Delete': () => {
                const { selection } = this.editor.state;
                return selection instanceof NodeSelection && selection.node.type.name === 'queryBlock';
            },
        };
    },

    addNodeView() {
        return ({ node, getPos, editor }) => new QueryBlockNodeView(node, getPos, editor);
    },

    // ── Markdown serialization ────────────────────────────────────────────────
    addStorage() {
        return {
            markdown: {
                serialize(state: MarkdownSerializerState, node: PmNode) {
                    state.write('```query\n');
                    if (node.attrs.locked) state.write('!locked\n');
                    const q: string = node.attrs.query ?? '';
                    if (q) state.write(q);
                    state.ensureNewLine();
                    state.write('```');
                    state.closeBlock(node);
                },
                parse: {
                    updateDOM(element: Element) {
                        element.querySelectorAll('pre > code.language-query').forEach((code) => {
                            const pre = code.parentElement!;
                            const raw = code.textContent?.trim() ?? '';
                            const locked = raw.startsWith('!locked\n') || raw === '!locked';
                            const query = locked ? raw.slice('!locked\n'.length).trim() : raw;
                            const div = document.createElement('div');
                            div.setAttribute('data-type', 'queryBlock');
                            div.setAttribute('data-query', query);
                            if (locked) div.setAttribute('data-locked', 'true');
                            pre.replaceWith(div);
                        });
                    },
                },
            },
        };
    },
});
