import { Node, mergeAttributes } from '@tiptap/core';
import { TextSelection } from '@tiptap/pm/state';
import { getDrawing, saveDrawing, getDrawingPreview, saveDrawingPreview, deleteDrawing, type ExcalidrawData } from './api';

// ── Lazy Excalidraw loader ─────────────────────────────────────────────────────

async function openDrawingEditor(
    name: string,
    isDark: boolean,
    onSaved: (svg: string) => void,
    onClose: () => void,
) {
    // Lazy-load React + Excalidraw (incl. CSS) so they don't bloat the initial bundle
    const [{ Excalidraw, exportToSvg }, { createElement: h }, { createRoot }] = await Promise.all([
        import('@excalidraw/excalidraw').then((m) => { import('@excalidraw/excalidraw/index.css'); return m; }),
        import('react'),
        import('react-dom/client'),
    ]);

    let initialData: ExcalidrawData | null = null;
    try { initialData = await getDrawing(name); } catch { /* new drawing */ }

    // Mutable refs for onChange values (closure, not React state — saves on unmount)
    let currentElements: unknown[] = initialData?.elements ?? [];
    let currentAppState: Record<string, unknown> = initialData?.appState ?? {};
    let currentFiles: Record<string, unknown> = initialData?.files ?? {};

    // ── Overlay ──────────────────────────────────────────────────────────────
    const overlay = document.createElement('div');
    overlay.className = 'drawing-overlay';
    document.body.appendChild(overlay);

    function cleanup() {
        root.unmount();
        overlay.remove();
    }

    async function handleSave() {
        const data: ExcalidrawData = {
            elements: currentElements,
            appState: currentAppState,
            files: currentFiles,
        };
        await saveDrawing(name, data);
        try {
            const svgEl = await exportToSvg({
                elements: currentElements as any,
                appState: { ...(currentAppState as any), exportBackground: true },
                files: currentFiles as any,
            });
            const svg = new XMLSerializer().serializeToString(svgEl);
            await saveDrawingPreview(name, svg);
            onSaved(svg);
        } catch (e) {
            console.warn('SVG export failed', e);
        }
        cleanup();
        onClose();
    }

    function handleClose() {
        cleanup();
        onClose();
    }

    // ── React tree (no JSX — createElement only) ─────────────────────────────
    const root = createRoot(overlay);
    root.render(
        h('div', { className: 'drawing-editor-root' },
            h('div', { className: 'drawing-editor-toolbar' },
                h('span', { className: 'drawing-editor-name' }, name),
                h('button', { className: 'drawing-editor-btn', onClick: handleClose }, 'Close'),
                h('button', { className: 'drawing-editor-btn primary', onClick: handleSave }, 'Save'),
            ),
            h('div', { className: 'drawing-editor-canvas' },
                h(Excalidraw as any, {
                    initialData: initialData ? {
                        elements: initialData.elements,
                        appState: { ...initialData.appState, collaborators: new Map(), theme: isDark ? 'dark' : 'light' },
                        files: initialData.files,
                    } : {
                        appState: { theme: isDark ? 'dark' : 'light' },
                    },
                    onChange: (elements: unknown[], appState: Record<string, unknown>, files: Record<string, unknown>) => {
                        currentElements = elements;
                        currentAppState = appState;
                        currentFiles = files;
                    },
                }),
            ),
        ),
    );
}

// ── TipTap node ────────────────────────────────────────────────────────────────

export const DrawingBlock = Node.create({
    name: 'drawingBlock',
    group: 'block',
    atom: true,
    draggable: true,

    addAttributes() {
        return {
            name: { default: '' },
            height: { default: 300 },
        };
    },

    parseHTML() {
        return [{
            tag: 'div[data-type="drawingBlock"]',
            getAttrs: (el) => ({
                name: (el as HTMLElement).getAttribute('data-name') ?? '',
                height: parseInt((el as HTMLElement).getAttribute('data-height') ?? '300', 10) || 300,
            }),
        }];
    },

    renderHTML({ HTMLAttributes }) {
        return ['div', mergeAttributes(HTMLAttributes, {
            'data-type': 'drawingBlock',
            'data-name': HTMLAttributes.name ?? '',
            'data-height': String(HTMLAttributes.height ?? 300),
            class: 'drawing-block-static',
        })];
    },

    addNodeView() {
        return ({ node, getPos, editor }) => {
            let currentNode = node;

            // ── Wrapper ────────────────────────────────────────────────────
            const dom = document.createElement('div');
            dom.className = 'drawing-block';
            dom.setAttribute('contenteditable', 'false');

            // ── Header ─────────────────────────────────────────────────────
            const header = document.createElement('div');
            header.className = 'drawing-block-header';

            const icon = document.createElement('span');
            icon.className = 'drawing-block-icon';
            icon.textContent = '✎';

            const nameEl = document.createElement('span');
            nameEl.className = 'drawing-block-name';
            nameEl.textContent = node.attrs.name || 'Drawing';

            const editBtn = document.createElement('button');
            editBtn.className = 'drawing-block-edit-btn';
            editBtn.textContent = 'Edit';

            const deleteBtn = document.createElement('button');
            deleteBtn.className = 'drawing-block-delete-btn';
            deleteBtn.textContent = '✕';
            deleteBtn.title = 'Remove drawing';

            header.append(icon, nameEl, editBtn, deleteBtn);
            dom.appendChild(header);

            // ── Preview ────────────────────────────────────────────────────
            const preview = document.createElement('div');
            preview.className = 'drawing-block-preview';
            preview.style.height = `${node.attrs.height ?? 300}px`;
            dom.appendChild(preview);

            let previewCtrl: AbortController | null = null;

            function setPreview(svg: string) {
                preview.innerHTML = svg;
                const svgEl = preview.querySelector('svg');
                if (svgEl) {
                    svgEl.setAttribute('width', '100%');
                    svgEl.setAttribute('height', '100%');
                    svgEl.style.display = 'block';
                }
            }

            function loadPreview(n: string) {
                previewCtrl?.abort();
                previewCtrl = new AbortController();
                getDrawingPreview(n, previewCtrl.signal)
                    .then(setPreview)
                    .catch((e) => {
                        if (e instanceof DOMException && e.name === 'AbortError') return;
                        preview.innerHTML = '<span class="drawing-placeholder">Click Edit to start drawing</span>';
                    });
            }

            loadPreview(node.attrs.name);

            // ── Resize handle ──────────────────────────────────────────────
            const resizeHandle = document.createElement('div');
            resizeHandle.className = 'drawing-resize-handle';
            resizeHandle.title = 'Drag to resize';
            dom.appendChild(resizeHandle);

            resizeHandle.addEventListener('mousedown', (e) => {
                e.preventDefault();
                e.stopPropagation();
                const startY = e.clientY;
                const startHeight = currentNode.attrs.height ?? 300;

                const onMove = (ev: MouseEvent) => {
                    const newH = Math.max(80, Math.min(1200, startHeight + ev.clientY - startY));
                    preview.style.height = `${newH}px`;
                };

                const onUp = (ev: MouseEvent) => {
                    document.removeEventListener('mousemove', onMove);
                    document.removeEventListener('mouseup', onUp);
                    const newH = Math.max(80, Math.min(1200, startHeight + ev.clientY - startY));
                    const pos = getPos();
                    if (typeof pos === 'number') {
                        editor.view.dispatch(
                            editor.view.state.tr.setNodeMarkup(pos, undefined, {
                                ...currentNode.attrs,
                                height: newH,
                            })
                        );
                    }
                };

                document.addEventListener('mousemove', onMove);
                document.addEventListener('mouseup', onUp);
            });

            // ── Cursor helpers (same pattern as queryBlock) ────────────────
            function moveCursorTo(pos: number) {
                const { state } = editor.view;
                const clamped = Math.max(0, Math.min(pos, state.doc.content.size));
                editor.view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, clamped)));
                editor.view.focus();
            }

            // ── Event handlers ─────────────────────────────────────────────
            editBtn.addEventListener('click', () => {
                const DARK_THEMES = new Set(['github-dark', 'dracula', 'rose-pine', 'desert']);
                const theme = document.documentElement.getAttribute('data-theme') ?? '';
                const isDark = DARK_THEMES.has(theme)
                    || (!theme && window.matchMedia('(prefers-color-scheme: dark)').matches);
                openDrawingEditor(currentNode.attrs.name, isDark, setPreview, () => {
                    editor.view.focus();
                });
            });

            deleteBtn.addEventListener('click', () => {
                const pos = getPos();
                deleteDrawing(currentNode.attrs.name).catch(() => {});
                if (typeof pos === 'number') {
                    editor.chain().deleteRange({ from: pos, to: pos + currentNode.nodeSize }).focus().run();
                }
            });

            dom.addEventListener('click', (e) => {
                const pos = getPos();
                if (typeof pos === 'number' && e.target === dom) moveCursorTo(pos);
            });

            return {
                dom,
                selectNode() { editBtn.focus(); },
                update(updatedNode) {
                    if (updatedNode.type.name !== 'drawingBlock') return false;
                    currentNode = updatedNode;
                    nameEl.textContent = updatedNode.attrs.name || 'Drawing';
                    preview.style.height = `${updatedNode.attrs.height ?? 300}px`;
                    return true;
                },
                destroy() {
                    previewCtrl?.abort();
                },
            };
        };
    },

    // ── Markdown serialization ─────────────────────────────────────────────
    addStorage() {
        return {
            markdown: {
                serialize(state: any, node: any) {
                    state.write('```drawing\n');
                    if (node.attrs.name) state.write(`${node.attrs.name} ${node.attrs.height ?? 300}`);
                    state.ensureNewLine();
                    state.write('```');
                    state.closeBlock(node);
                },
                parse: {
                    updateDOM(element: Element) {
                        element.querySelectorAll('pre > code.language-drawing').forEach((code) => {
                            const pre = code.parentElement!;
                            const text = code.textContent?.trim() ?? '';
                            const spaceIdx = text.lastIndexOf(' ');
                            const name = spaceIdx > 0 ? text.slice(0, spaceIdx) : text;
                            const height = spaceIdx > 0 ? parseInt(text.slice(spaceIdx + 1), 10) || 300 : 300;
                            const div = document.createElement('div');
                            div.setAttribute('data-type', 'drawingBlock');
                            div.setAttribute('data-name', name);
                            div.setAttribute('data-height', String(height));
                            pre.replaceWith(div);
                        });
                    },
                },
            },
        };
    },
});
