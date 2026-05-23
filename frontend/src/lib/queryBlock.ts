import { Node, mergeAttributes } from '@tiptap/core';
import { NodeSelection, TextSelection } from '@tiptap/pm/state';
import { queryNotes, listTags, getFieldValues, type NoteQueryResult, type TagCount } from './api';
import { emit, on } from './events';

// ── Autocomplete token detection ──────────────────────────────────────────────

interface ActiveToken {
    type: 'tag' | 'field';
    field: string;
    partial: string;
    start: number;
    end: number;
}

const COMPLETABLE_FIELDS = new Set(['status', 'area', 'author', 'type', 'due', 'rating', 'tag']);

function getActiveToken(el: HTMLInputElement): ActiveToken | null {
    const pos = el.selectionStart ?? el.value.length;
    const val = el.value;
    let s = pos;
    while (s > 0 && val[s - 1] !== ' ') s--;
    let e = pos;
    while (e < val.length && val[e] !== ' ') e++;
    const tok = val.slice(s, e);
    if (!tok || tok === 'AND' || tok === 'OR' || tok === 'NOT') return null;
    if (tok.startsWith('#')) {
        return { type: 'tag', field: 'tag', partial: tok.slice(1), start: s, end: e };
    }
    const ci = tok.indexOf(':');
    if (ci > 0) {
        const field = tok.slice(0, ci).toLowerCase();
        if (COMPLETABLE_FIELDS.has(field)) {
            return { type: 'field', field, partial: tok.slice(ci + 1), start: s, end: e };
        }
    }
    return null;
}

// ── Print directive ───────────────────────────────────────────────────────────

interface PrintSpec {
    fields: string[];
}

// Matches "print field1 field2" at the end — only valid field names, so "order by" isn't captured.
const PRINT_FIELDS_PAT = '(?:name|path|title|tags|date|status|area|author|due|rating|url)';
const PRINT_RE = new RegExp(`\\bprint\\s+(${PRINT_FIELDS_PAT}(?:[\\s,]+${PRINT_FIELDS_PAT})*)\\s*$`, 'i');

function parsePrint(q: string): { cleanQuery: string; print: PrintSpec | null } {
    const m = q.match(PRINT_RE);
    if (!m) return { cleanQuery: q, print: null };
    const fields = m[1].split(/[\s,]+/).filter(Boolean).map(f => f.toLowerCase());
    return {
        cleanQuery: q.slice(0, m.index).trim(),
        print: fields.length ? { fields } : null,
    };
}

const PRINT_FIELDS = new Set(['name', 'path', 'title', 'tags', 'date', 'status', 'area', 'author', 'due', 'rating', 'url']);

function getFieldText(field: string, row: NoteQueryResult): string {
    switch (field) {
        case 'name':   return row.name.split('/').pop() ?? row.name;
        case 'path':   return row.name;
        case 'title':  return row.title || row.name;
        case 'tags':   return row.tags.join(', ');
        case 'date':   return row.date ?? '';
        case 'status': return row.status ?? '';
        case 'area':   return row.area ?? '';
        case 'author': return row.author ?? '';
        case 'due':    return row.due ?? '';
        case 'rating': return row.rating != null ? String(row.rating) : '';
        case 'url':    return row.url ?? '';
        default:       return '';
    }
}

// ── SVG icons ─────────────────────────────────────────────────────────────────

const ICON_LOCKED = `<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="7" width="10" height="8" rx="1.5"/><path d="M5 7V5a3 3 0 0 1 6 0v2"/></svg>`;
const ICON_UNLOCKED = `<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="7" width="10" height="8" rx="1.5"/><path d="M5 7V5a3 3 0 0 1 5.83 1"/></svg>`;

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
            // The user can still delete it intentionally by clearing the input and pressing Backspace.
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
        return ({ node, getPos, editor }) => {
            let currentNode = node;
            let currentPrint: PrintSpec | null = null;

            // ── DOM ─────────────────────────────────────────────────────────
            const dom = document.createElement('div');
            dom.className = 'query-block';
            dom.setAttribute('contenteditable', 'false');

            const header = document.createElement('div');
            header.className = 'query-block-header';

            const icon = document.createElement('span');
            icon.className = 'query-block-icon';
            icon.textContent = '{}';

            const input = document.createElement('input');
            input.className = 'query-block-input';
            input.placeholder = '#tag  status:active  order by date desc  print title';
            input.value = node.attrs.query ?? '';
            input.spellcheck = false;

            const countEl = document.createElement('span');
            countEl.className = 'query-result-count';

            const lockBtn = document.createElement('button');
            lockBtn.type = 'button';
            lockBtn.className = 'query-block-lock';
            lockBtn.title = 'Lock query';

            header.appendChild(icon);
            header.appendChild(input);
            header.appendChild(countEl);
            header.appendChild(lockBtn);
            dom.appendChild(header);

            const results = document.createElement('div');
            results.className = 'query-block-results';
            dom.appendChild(results);

            results.addEventListener('keydown', (e) => {
                const items = Array.from(results.querySelectorAll<HTMLElement>('.query-result-name'));
                const idx = items.indexOf(document.activeElement as HTMLElement);
                if (e.key === 'ArrowDown') {
                    if (idx < items.length - 1) {
                        items[idx + 1].focus();
                    } else {
                        const pmPos = getPos();
                        if (typeof pmPos === 'number') moveCursorTo(pmPos + currentNode.nodeSize);
                    }
                    e.preventDefault(); e.stopPropagation();
                } else if (e.key === 'ArrowUp') {
                    if (idx > 0) items[idx - 1].focus();
                    else input.focus();
                    e.preventDefault(); e.stopPropagation();
                } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                    insertParagraphBelow();
                    e.preventDefault(); e.stopPropagation();
                } else if (e.key === 'Enter') {
                    const active = document.activeElement as HTMLElement;
                    if (items.includes(active)) active.click();
                    e.preventDefault(); e.stopPropagation();
                } else if (e.key === 'Escape') {
                    input.focus();
                    e.preventDefault(); e.stopPropagation();
                }
            });

            // ── Lock state ───────────────────────────────────────────────────
            function applyLocked(locked: boolean) {
                input.readOnly = locked;
                lockBtn.innerHTML = locked ? ICON_LOCKED : ICON_UNLOCKED;
                lockBtn.title = locked ? 'Unlock query' : 'Lock query';
                dom.classList.toggle('locked', locked);
            }

            applyLocked(node.attrs.locked ?? false);

            // Intercept mousedown so ProseMirror never steals focus from the input.
            dom.addEventListener('mousedown', (e) => {
                const target = e.target as HTMLElement;
                if (target === input) {
                    e.stopPropagation(); // let the browser focus the input naturally
                } else if (!target.closest('button')) {
                    e.preventDefault(); // prevent PM from creating a NodeSelection
                    input.focus();
                }
            });

            lockBtn.addEventListener('click', () => {
                const pos = getPos();
                if (typeof pos !== 'number') return;
                const newLocked = !currentNode.attrs.locked;
                editor.view.dispatch(
                    editor.view.state.tr.setNodeMarkup(pos, undefined, {
                        ...currentNode.attrs,
                        locked: newLocked,
                    })
                );
            });

            // ── Helpers ──────────────────────────────────────────────────────
            function moveCursorTo(pos: number) {
                const { state } = editor.view;
                const clamped = Math.max(0, Math.min(pos, state.doc.content.size));
                editor.view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, clamped)));
                editor.view.focus();
            }

            function insertParagraphBelow() {
                const pmPos = getPos();
                if (typeof pmPos === 'number') {
                    const insertAt = pmPos + currentNode.nodeSize;
                    editor.chain()
                        .insertContentAt(insertAt, { type: 'paragraph' })
                        .setTextSelection(insertAt + 1)
                        .focus()
                        .run();
                }
            }

            function updateQueryAttr() {
                if (currentNode.attrs.locked) return;
                const pos = getPos();
                if (typeof pos === 'number') {
                    editor.view.dispatch(
                        editor.view.state.tr.setNodeMarkup(pos, undefined, {
                            ...currentNode.attrs,
                            query: input.value,
                        })
                    );
                }
            }

            function makeLink(text: string, row: NoteQueryResult): HTMLButtonElement {
                const btn = document.createElement('button');
                btn.type = 'button';
                btn.className = 'query-result-name';
                btn.textContent = text;
                btn.addEventListener('click', () => {
                    emit(document, 'wiki-navigate', row.name);
                });
                return btn;
            }

            // ── Autocomplete ─────────────────────────────────────────────────
            let acSuggestions: string[] = [];
            let acIndex = -1;
            let acToken: ActiveToken | null = null;
            let acRows: NoteQueryResult[] = [];

            function stripToken(query: string, token: ActiveToken): string {
                const before = query.slice(0, token.start).replace(/\s*(AND|OR|NOT)\s*$/, '').trimEnd();
                const after = query.slice(token.end).replace(/^\s*(AND|OR|NOT)\s*/, '').trimStart();
                if (!before && !after) return '';
                if (!before) return after;
                if (!after) return before;
                return before + ' ' + after;
            }

            function renderBrowseMode() {
                results.innerHTML = '';
                countEl.textContent = acRows.length ? `${acRows.length}` : '';
                if (acSuggestions.length > 0 && acToken) {
                    const cloud = document.createElement('div');
                    cloud.className = 'query-tag-cloud' + (acRows.length ? ' query-field-cloud' : '');
                    acSuggestions.forEach((item, i) => {
                        const chip = document.createElement('button');
                        chip.className = 'query-tag-chip' + (i === acIndex ? ' active' : '');
                        const label = document.createElement('span');
                        label.className = 'query-tag-name';
                        label.textContent = acToken!.type === 'tag' ? `#${item}` : item;
                        chip.appendChild(label);
                        chip.addEventListener('mousedown', (e) => {
                            e.preventDefault();
                            applySuggestion(item);
                        });
                        cloud.appendChild(chip);
                    });
                    results.appendChild(cloud);
                }
                if (acRows.length > 0) appendResultRows(results, acRows, currentPrint);
            }

            function applySuggestion(value: string) {
                if (!acToken) return;
                const completion = acToken.type === 'tag'
                    ? `#${value}`
                    : `${acToken.field}:${value}`;
                const before = input.value.slice(0, acToken.start);
                const after = input.value.slice(acToken.end);
                const sep = after.length > 0 && after[0] !== ' ' ? ' ' : '';
                input.value = before + completion + sep + after;
                const newPos = acToken.start + completion.length + sep.length;
                input.setSelectionRange(newPos, newPos);
                acSuggestions = [];
                acIndex = -1;
                acToken = null;
                updateQueryAttr();
                fetchResults(input.value);
            }

            async function checkAutocompletion() {
                const token = getActiveToken(input);
                if (!token || token.partial) {
                    acSuggestions = [];
                    acIndex = -1;
                    acToken = null;
                    acRows = [];
                    fetchResults(input.value);
                    return;
                }
                let chipItems: string[];
                if (token.type === 'tag' || token.field === 'tag') {
                    const tags = await listTags();
                    chipItems = tags.map(t => t.tag);
                } else {
                    chipItems = await getFieldValues(token.field);
                }
                const baseQuery = stripToken(input.value, token);
                const { cleanQuery } = parsePrint(baseQuery);
                const rows = cleanQuery ? await queryNotes(cleanQuery) : [];
                acSuggestions = chipItems;
                acIndex = -1;
                acToken = token;
                acRows = rows;
                renderBrowseMode();
            }

            // ── Keyboard ─────────────────────────────────────────────────────
            input.addEventListener('keydown', (e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                    insertParagraphBelow();
                    e.preventDefault(); e.stopPropagation(); return;
                }
                if (acSuggestions.length > 0) {
                    if (e.key === 'ArrowDown') {
                        acIndex = Math.min(acIndex + 1, acSuggestions.length - 1);
                        renderBrowseMode();
                        e.preventDefault(); e.stopPropagation(); return;
                    }
                    if (e.key === 'ArrowUp') {
                        acIndex = Math.max(acIndex - 1, 0);
                        renderBrowseMode();
                        e.preventDefault(); e.stopPropagation(); return;
                    }
                    if (e.key === 'Tab') {
                        applySuggestion(acIndex >= 0 ? acSuggestions[acIndex] : acSuggestions[0]);
                        e.preventDefault(); e.stopPropagation(); return;
                    }
                    if (e.key === 'Escape') {
                        acSuggestions = [];
                        acIndex = -1;
                        acToken = null;
                        fetchResults(input.value);
                        e.preventDefault(); e.stopPropagation(); return;
                    }
                }

                const pmPos = getPos();
                switch (e.key) {
                    case 'ArrowUp':
                        if (typeof pmPos === 'number') moveCursorTo(pmPos);
                        e.preventDefault(); e.stopPropagation(); break;
                    case 'ArrowDown': {
                        const firstBtn = results.querySelector<HTMLElement>('.query-result-name');
                        if (firstBtn) {
                            firstBtn.focus();
                        } else if (typeof pmPos === 'number') {
                            moveCursorTo(pmPos + currentNode.nodeSize);
                        }
                        e.preventDefault(); e.stopPropagation(); break;
                    }
                    case 'Enter': {
                        insertParagraphBelow();
                        e.preventDefault(); e.stopPropagation(); break;
                    }
                    case 'Escape':
                        editor.view.focus();
                        e.stopPropagation(); break;
                    case 'Backspace': {
                        const atStart = input.selectionStart === 0 && input.selectionEnd === 0;
                        if ((input.value === '' || atStart) && !currentNode.attrs.locked && typeof pmPos === 'number') {
                            editor.chain()
                                .deleteRange({ from: pmPos, to: pmPos + currentNode.nodeSize })
                                .focus()
                                .run();
                            e.preventDefault();
                        }
                        e.stopPropagation(); break;
                    }
                    default:
                        e.stopPropagation(); break;
                }
            });

            const stop = (e: Event) => e.stopPropagation();
            input.addEventListener('keyup', stop);
            input.addEventListener('keypress', stop);
            input.addEventListener('beforeinput', stop);

            // ── Results rendering ────────────────────────────────────────────
            let debounceTimer: ReturnType<typeof setTimeout> | null = null;

            function renderTags(tags: TagCount[]) {
                results.innerHTML = '';
                countEl.textContent = tags.length ? `${tags.length}` : '';
                if (!tags.length) {
                    results.innerHTML = '<span class="query-empty">No tags found</span>';
                    return;
                }
                const cloud = document.createElement('div');
                cloud.className = 'query-tag-cloud';
                tags.forEach(({ tag, count }) => {
                    const chip = document.createElement('button');
                    chip.className = 'query-tag-chip';
                    const tagNameEl = document.createElement('span');
                    tagNameEl.className = 'query-tag-name';
                    tagNameEl.textContent = tag;
                    const tagCountEl = document.createElement('span');
                    tagCountEl.className = 'query-tag-count';
                    tagCountEl.textContent = String(count);
                    chip.append(tagNameEl, tagCountEl);
                    chip.addEventListener('click', () => {
                        const newQ = `#${tag}`;
                        input.value = newQ;
                        const pos = getPos();
                        if (typeof pos === 'number') {
                            editor.view.dispatch(
                                editor.view.state.tr.setNodeMarkup(pos, undefined, {
                                    ...currentNode.attrs,
                                    query: newQ,
                                })
                            );
                        }
                        fetchResults(newQ);
                    });
                    cloud.appendChild(chip);
                });
                results.appendChild(cloud);
            }

            function appendResultRows(container: HTMLElement, rows: NoteQueryResult[], print: PrintSpec | null) {
                if (!rows.length) {
                    const empty = document.createElement('span');
                    empty.className = 'query-empty';
                    empty.textContent = 'No results';
                    container.appendChild(empty);
                    return;
                }
                rows.forEach((row) => {
                    const item = document.createElement('div');
                    item.className = 'query-result-item';

                    if (print) {
                        let hasLink = false;
                        print.fields.forEach((field) => {
                            const isLink = field === 'name' || field === 'path' || field === 'title';
                            if (isLink) {
                                const text = field === 'title' ? (row.title || row.name) : field === 'path' ? row.name : (row.name.split('/').pop() ?? row.name);
                                item.appendChild(makeLink(text, row));
                                hasLink = true;
                            } else if (PRINT_FIELDS.has(field)) {
                                const text = getFieldText(field, row);
                                if (text) {
                                    const span = document.createElement('span');
                                    span.className = 'query-result-meta';
                                    span.textContent = text;
                                    item.appendChild(span);
                                }
                            }
                        });
                        // If no navigable field was requested, make the row itself a link
                        if (!hasLink) {
                            const btn = makeLink(getFieldText(print.fields[0], row) || row.name, row);
                            btn.className = 'query-result-name';
                            item.prepend(btn);
                        }
                    } else {
                        item.appendChild(makeLink(row.title || row.name, row));
                        if (row.title && row.title !== row.name) {
                            const pathEl = document.createElement('span');
                            pathEl.className = 'query-result-path';
                            pathEl.textContent = row.name;
                            item.appendChild(pathEl);
                        }
                        const metaParts: string[] = [];
                        if (row.tags.length) metaParts.push(row.tags.join(', '));
                        if (row.date) metaParts.push(row.date);
                        if (row.status) metaParts.push(row.status);
                        if (metaParts.length) {
                            const meta = document.createElement('span');
                            meta.className = 'query-result-meta';
                            meta.textContent = metaParts.join(' · ');
                            item.appendChild(meta);
                        }
                    }

                    container.appendChild(item);
                });
            }

            function renderResults(rows: NoteQueryResult[], print: PrintSpec | null) {
                results.innerHTML = '';
                countEl.textContent = rows.length ? `${rows.length}` : '';
                appendResultRows(results, rows, print);
            }

            async function fetchResults(q: string) {
                const { cleanQuery, print } = parsePrint(q);
                currentPrint = print;

                if (!cleanQuery.trim()) {
                    countEl.textContent = '';
                    results.innerHTML = '<span class="query-empty">Type a query above…</span>';
                    return;
                }
                results.innerHTML = '<span class="query-empty">Searching…</span>';
                try {
                    if (cleanQuery.trim() === '#') {
                        const tags = await listTags();
                        renderTags(tags);
                        return;
                    }
                    const rows = await queryNotes(cleanQuery);
                    renderResults(rows, print);
                } catch {
                    countEl.textContent = '';
                    results.innerHTML = '<span class="query-empty query-error">Query error</span>';
                }
            }

            input.addEventListener('input', () => {
                if (currentNode.attrs.locked) return;
                if (debounceTimer) clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    updateQueryAttr();
                    checkAutocompletion();
                }, 200);
            });

            fetchResults(node.attrs.query ?? '');

            const offNotesChanged = on(document, 'notes:changed',
                () => fetchResults(currentNode.attrs.query ?? '')
            );

            return {
                dom,
                selectNode() {
                    // setTimeout: ProseMirror calls view.focus() after selectNode(),
                    // which would steal focus back from the input without this delay.
                    setTimeout(() => input.focus(), 0);
                },
                update(updatedNode) {
                    if (updatedNode.type.name !== 'queryBlock') return false;
                    currentNode = updatedNode;
                    applyLocked(updatedNode.attrs.locked ?? false);
                    const newQ = updatedNode.attrs.query ?? '';
                    if (input.value !== newQ) {
                        input.value = newQ;
                        fetchResults(newQ);
                    }
                    return true;
                },
                destroy() {
                    if (debounceTimer) clearTimeout(debounceTimer);
                    offNotesChanged();
                },
            };
        };
    },

    // ── Markdown serialization ────────────────────────────────────────────────
    addStorage() {
        return {
            markdown: {
                serialize(state: any, node: any) {
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
