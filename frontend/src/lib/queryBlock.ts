import { Node, mergeAttributes } from '@tiptap/core';
import { NodeSelection, TextSelection } from '@tiptap/pm/state';
import { queryNotes, listTags, getFieldValues, type NoteQueryResult, type TagCount } from './api';
import { on } from './events';
import {
    getActiveToken, parsePrint, ICON_LOCKED, ICON_UNLOCKED,
    type ActiveToken, type PrintSpec,
} from './queryBlockHelpers';
import { makeLink, appendResultRows, buildTagCloud, buildTagCountCloud } from './queryBlockDom';

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

            dom.addEventListener('mousedown', (e) => {
                const target = e.target as HTMLElement;
                if (target === input) {
                    e.stopPropagation();
                } else if (!target.closest('button')) {
                    e.preventDefault();
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
                    const tagCounts: TagCount[] = acSuggestions.map(s => ({ tag: s, count: 0 }));
                    const cloud = buildTagCloud(
                        tagCounts,
                        (tag) => applySuggestion(tag),
                        acIndex,
                        acRows.length > 0,
                        acToken.type === 'tag' ? '#' : '',
                    );
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
                const cloud = buildTagCountCloud(tags, (tag) => {
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
                results.appendChild(cloud);
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
