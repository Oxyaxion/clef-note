<script lang="ts">
    import { untrack, tick } from 'svelte';
    import type { Frontmatter } from './api';
    import { escapeHtml } from './utils';

    interface Props {
        frontmatter: Frontmatter;
        onChange: (fm: Frontmatter) => void;
    }

    let { frontmatter, onChange }: Props = $props();

    const hasContent = $derived(
        Object.entries(frontmatter).some(([, v]) =>
            v !== null && v !== undefined && v !== '' &&
            !(Array.isArray(v) && v.length === 0)
        )
    );

    let expanded   = $state(false);
    let rawYaml    = $state(untrack(() => toYaml(frontmatter)));
    let parseError = $state('');
    let isFocused  = $state(false);
    let menuOpen   = $state(false);
    let menuEl     = $state<HTMLDivElement | null>(null);
    let addBtnEl   = $state<HTMLButtonElement | null>(null);
    let textareaEl = $state<HTMLTextAreaElement | null>(null);
    // Plain (non-reactive) sentinel: canonical YAML of the last value we sent via onChange.
    // Lets the effect tell apart "frontmatter prop echoed our own save" (no-op)
    // from "external change — note switch, H1 auto-sync, …" (must reset rawYaml).
    let _lastSaved = untrack(() => toYaml(frontmatter));

    // Sync external frontmatter changes (note switch, …) into the YAML editor.
    // When the change came from our own onBlur → onChange, yaml === _lastSaved → no-op.
    $effect(() => {
        const yaml = toYaml(frontmatter);
        if (!isFocused && yaml !== _lastSaved) {
            parseError = '';
            rawYaml = yaml;
            _lastSaved = yaml;
        }
    });

    // ── Field suggestions ─────────────────────────────────────────────────────

    type FieldDef = { key: string; hint: string; isArray: boolean; default: string | (() => string) };

    const FIELDS: FieldDef[] = [
        { key: 'title',   hint: 'Mon titre',        isArray: false, default: '' },
        { key: 'type',    hint: 'note',              isArray: false, default: 'note' },
        { key: 'status',  hint: 'active',           isArray: false, default: 'active' },
        { key: 'date',    hint: 'YYYY-MM-DD',       isArray: false, default: () => new Date().toISOString().split('T')[0] },
        { key: 'due',     hint: 'YYYY-MM-DD',       isArray: false, default: () => new Date().toISOString().split('T')[0] },
        { key: 'tags',    hint: '- tag1',           isArray: true,  default: '' },
        { key: 'pinned',  hint: 'true / false',     isArray: false, default: 'false' },
        { key: 'area',    hint: '…',                isArray: false, default: '' },
        { key: 'url',     hint: 'https://…',        isArray: false, default: '' },
        { key: 'author',  hint: '…',                isArray: false, default: '' },
        { key: 'rating',  hint: '1-5',              isArray: false, default: '' },
        { key: 'aliases', hint: '- autre-nom',      isArray: true,  default: '' },
        { key: 'toc',     hint: 'true / false',     isArray: false, default: 'false' },
    ];

    const availableFields = $derived(
        FIELDS.filter(f => !new RegExp(`^${f.key}:`, 'm').test(rawYaml))
    );

    async function insertField(f: FieldDef) {
        menuOpen = false;
        const val = typeof f.default === 'function' ? f.default() : f.default;
        const line = f.isArray ? `${f.key}:\n  - ` : `${f.key}: ${val}`;
        rawYaml = rawYaml ? rawYaml + '\n' + line : line;
        expanded = true;
        await tick();
        if (textareaEl) {
            textareaEl.style.height = 'auto';
            textareaEl.style.height = textareaEl.scrollHeight + 'px';
            textareaEl.focus();
            textareaEl.setSelectionRange(textareaEl.value.length, textareaEl.value.length);
        }
    }

    function handleWindowClick(e: MouseEvent) {
        if (!menuOpen) return;
        const t = e.target as Node;
        if (addBtnEl?.contains(t) || menuEl?.contains(t)) return;
        menuOpen = false;
    }

    // ── YAML serialiser ───────────────────────────────────────────────────────

    function yamlScalar(v: unknown): string {
        if (typeof v !== 'string') return String(v);
        if (/[:#\[\]{}&*!|>'"%@`,]/.test(v) || v.trim() !== v || v === '')
            return `"${v.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
        return v;
    }

    function toYaml(fm: Frontmatter): string {
        const entries = Object.entries(fm).filter(([, v]) =>
            v !== null && v !== undefined && v !== '' &&
            !(Array.isArray(v) && v.length === 0)
        );
        if (!entries.length) return '';
        return entries.map(([key, value]) =>
            Array.isArray(value)
                ? `${key}:\n${(value as unknown[]).map(v => `  - ${yamlScalar(v)}`).join('\n')}`
                : `${key}: ${yamlScalar(value)}`
        ).join('\n');
    }

    // ── YAML parser (our subset: scalars + string arrays) ────────────────────

    // Fields that are always arrays — a bare scalar value "a, b" is auto-split.
    const ARRAY_FIELDS = new Set(['tags', 'aliases']);

    function parseScalar(v: string): unknown {
        if (v === 'true')  return true;
        if (v === 'false') return false;
        if (v === 'null' || v === '~') return null;
        if (/^-?\d+$/.test(v))      return parseInt(v, 10);
        if (/^-?\d+\.\d+$/.test(v)) return parseFloat(v);
        if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'")))
            return v.slice(1, -1).replace(/\\(["\\])/g, '$1');
        return v;
    }

    function fromYaml(yaml: string): Frontmatter | null {
        try {
            const result: Frontmatter = {};
            const lines = yaml.split('\n');
            let i = 0;
            while (i < lines.length) {
                const line = lines[i];
                if (!line.trim() || line.trim().startsWith('#')) { i++; continue; }

                const inlineArr = line.match(/^([a-zA-Z_][\w-]*):\s*\[(.*)\]\s*$/);
                if (inlineArr) {
                    result[inlineArr[1]] = inlineArr[2].split(',')
                        .map(s => parseScalar(s.trim()))
                        .filter(v => v !== undefined && v !== '');
                    i++; continue;
                }
                const blockKey = line.match(/^([a-zA-Z_][\w-]*):\s*$/);
                if (blockKey) {
                    const key = blockKey[1]; i++;
                    const items: unknown[] = [];
                    // Accept any indentation and optional space after dash (auto-corrected on blur)
                    while (i < lines.length && /^\s*-/.test(lines[i])) {
                        const v = parseScalar(lines[i++].replace(/^\s*-\s*/, '').trim());
                        if (v !== undefined && v !== '') items.push(v);
                    }
                    if (items.length) result[key] = items;
                    continue;
                }
                const kv = line.match(/^([a-zA-Z_][\w-]*):\s*(.*)/);
                if (kv) {
                    const key = kv[1];
                    let val: unknown = parseScalar(kv[2].trim());
                    // "tags: rust, svelte" → auto-split into array
                    if (ARRAY_FIELDS.has(key) && typeof val === 'string' && val.includes(',')) {
                        const parts = val.split(',').map(s => parseScalar(s.trim())).filter(v => v !== undefined && v !== '');
                        if (parts.length > 0) val = parts;
                    }
                    if (val !== undefined && val !== null && val !== '') result[key] = val;
                    i++; continue;
                }
                i++;
            }
            return result;
        } catch { return null; }
    }

    // ── Syntax highlighter ────────────────────────────────────────────────────

    function highlightValue(val: string): string {
        if (!val) return '';
        if (val.startsWith('[') && val.endsWith(']')) {
            const inner = val.slice(1, -1);
            return `<span class="fy-p">[</span>${escapeHtml(inner)}<span class="fy-p">]</span>`;
        }
        if (val === 'true' || val === 'false')  return `<span class="fy-bool">${val}</span>`;
        if (val === 'null' || val === '~')       return `<span class="fy-null">${val}</span>`;
        if (/^-?\d+(\.\d+)?$/.test(val))        return `<span class="fy-num">${val}</span>`;
        return `<span class="fy-str">${escapeHtml(val)}</span>`;
    }

    function highlightYaml(yaml: string): string {
        if (!yaml) return '';
        return yaml.split('\n').map(line => {
            if (/^\s*#/.test(line))
                return `<span class="fy-comment">${escapeHtml(line)}</span>`;

            const arr = line.match(/^(\s*-\s+)(.*)$/);
            if (arr)
                return `<span class="fy-p">${escapeHtml(arr[1])}</span>${highlightValue(arr[2])}`;

            const kv = line.match(/^([a-zA-Z_][\w-]*)(:\s*)(.*)$/);
            if (kv)
                return `<span class="fy-key">${escapeHtml(kv[1])}</span><span class="fy-p">${escapeHtml(kv[2])}</span>${highlightValue(kv[3])}`;

            return escapeHtml(line);
        }).join('\n');
    }

    // ── Event handlers ────────────────────────────────────────────────────────

    function onBlur() {
        const parsed = fromYaml(rawYaml);
        if (parsed === null) { parseError = 'Invalid YAML'; return; }
        parseError = '';
        const normalized = toYaml(parsed);
        rawYaml = normalized;
        _lastSaved = normalized;
        onChange(parsed);
    }

    function initEl(el: HTMLElement) {
        el.style.height = 'auto';
        el.style.height = el.scrollHeight + 'px';
        el.focus();
    }
    function onInput(e: Event) {
        const el = e.target as HTMLTextAreaElement;
        el.style.height = 'auto';
        el.style.height = el.scrollHeight + 'px';
    }
</script>

<svelte:window onclick={handleWindowClick} />

<div class="fm-wrap">
    {#if !expanded}
        <button
            class="fm-pill-btn"
            onclick={() => { expanded = true; }}
        >
            <span class="fm-brace">{'{}'}</span>
            {#if hasContent}
                <span class="fm-pill-label">frontmatter</span>
            {:else}
                <span class="fm-pill-hint">Add metadata…</span>
            {/if}
        </button>
    {:else}
        <div class="fm-block">
            <div
                class="fm-header"
                role="button"
                tabindex="0"
                onclick={() => (expanded = false)}
                onkeydown={(e) => e.key === 'Enter' && (expanded = false)}
            >
                <span class="fm-header-label">{'{}'} frontmatter</span>
                {#if parseError}<span class="fm-error">{parseError}</span>{/if}

                <div class="fm-header-actions" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                    {#if availableFields.length > 0}
                        <button
                            bind:this={addBtnEl}
                            class="fm-add-btn"
                            onclick={() => (menuOpen = !menuOpen)}
                            title="Add field"
                        >+ field</button>
                    {/if}
                </div>

                {#if menuOpen}
                    <div bind:this={menuEl} class="fm-add-menu" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                        {#each availableFields as field (field.key)}
                            <button class="fm-add-item" onclick={() => insertField(field)}>
                                <span class="fm-add-key">{field.key}</span>
                                <span class="fm-add-hint">{field.hint}</span>
                            </button>
                        {/each}
                    </div>
                {/if}
            </div>

            <!-- Overlay editor: highlight div + transparent textarea in same grid cell -->
            <div class="fm-editor">
                <div class="fm-hl" aria-hidden="true">{@html highlightYaml(rawYaml)}</div>
                <textarea
                    bind:this={textareaEl}
                    class="fm-textarea"
                    class:has-content={rawYaml.length > 0}
                    bind:value={rawYaml}
                    onfocus={() => (isFocused = true)}
                    onblur={() => { isFocused = false; onBlur(); }}
                    oninput={onInput}
                    use:initEl
                    spellcheck={false}
                    autocomplete="off"
                    autocapitalize="off"
                    placeholder="title: Ma note&#10;tags:&#10;  - exemple&#10;type: note&#10;pinned: true"
                ></textarea>
            </div>
        </div>
    {/if}
</div>

<style>
    .fm-wrap {
        flex-shrink: 0;
        border-bottom: 1px solid var(--border);
        background: var(--sidebar-bg);
    }

    /* ── Collapsed pill ────────────────────────────────────────────── */
    .fm-pill-btn {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        width: 100%;
        background: none;
        border: none;
        cursor: pointer;
        padding: 0.25rem 0.65rem;
        font-family: inherit;
        transition: background 80ms;
    }
    .fm-pill-btn:hover {
        background: color-mix(in srgb, var(--border) 40%, transparent);
    }
    .fm-brace {
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
        font-size: 0.75rem;
        font-weight: 700;
        color: var(--muted);
    }
    .fm-pill-label { font-size: 0.78rem; color: var(--muted); }
    .fm-pill-hint  { font-size: 0.78rem; color: var(--muted); opacity: 0.55; }
    .fm-pill-btn:hover .fm-brace,
    .fm-pill-btn:hover .fm-pill-label,
    .fm-pill-btn:hover .fm-pill-hint { color: var(--text); opacity: 1; }

    /* ── Expanded block ────────────────────────────────────────────── */
    .fm-block {
        border: 1px solid var(--border);
        border-radius: 8px;
        overflow: visible;   /* allow dropdown to overflow */
        background: var(--sidebar-bg);

        /* Syntax colour tokens — light theme */
        --fy-key:     #005cc5;
        --fy-str:     #032f62;
        --fy-bool:    #d73a49;
        --fy-num:     #005cc5;
        --fy-null:    #d73a49;
        --fy-comment: #6a737d;
        --fy-p:       var(--muted);
    }

    /* Dark mode overrides */
    @media (prefers-color-scheme: dark) {
        .fm-block {
            --fy-key:     #79c0ff;
            --fy-str:     #a5d6ff;
            --fy-bool:    #ff7b72;
            --fy-num:     #79c0ff;
            --fy-null:    #ff7b72;
            --fy-comment: #8b949e;
        }
    }
    :global([data-theme="github-dark"]) .fm-block {
        --fy-key:     #79c0ff;
        --fy-str:     #a5d6ff;
        --fy-bool:    #ff7b72;
        --fy-num:     #79c0ff;
        --fy-null:    #ff7b72;
        --fy-comment: #8b949e;
    }

    :global([data-theme="solarized"]) .fm-block {
        --fy-key:     #268bd2;   /* blue */
        --fy-str:     #2aa198;   /* cyan */
        --fy-bool:    #859900;   /* green */
        --fy-num:     #d33682;   /* magenta */
        --fy-null:    #cb4b16;   /* orange */
        --fy-comment: #93a1a1;   /* base1 */
    }

    :global([data-theme="catppuccin"]) .fm-block {
        --fy-key:     #209fb5;   /* sapphire */
        --fy-str:     #40a02b;   /* green */
        --fy-bool:    #df8e1d;   /* yellow */
        --fy-num:     #7287fd;   /* lavender */
        --fy-null:    #8839ef;   /* mauve */
        --fy-comment: #8c8fa1;   /* overlay */
    }

    .fm-header {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.25rem 0.65rem;
        border-bottom: 1px solid var(--border);
        border-radius: 8px 8px 0 0;
        background: color-mix(in srgb, var(--border) 50%, transparent);
        cursor: pointer;
        transition: background 80ms;
    }
    .fm-header:hover {
        background: color-mix(in srgb, var(--border) 80%, transparent);
    }
    .fm-header-label {
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
        font-size: 0.72rem;
        font-weight: 700;
        color: var(--muted);
        flex: 1;
    }
    .fm-error { font-size: 0.72rem; color: #e57373; }

    .fm-header-actions {
        display: flex;
        align-items: center;
        gap: 0.4rem;
    }

    .fm-add-btn {
        background: none;
        border: 1px solid var(--border);
        border-radius: 4px;
        cursor: pointer;
        color: var(--muted);
        font-size: 0.7rem;
        font-family: inherit;
        padding: 0.1rem 0.4rem;
        line-height: 1.4;
        transition: background 80ms, color 80ms;
    }
    .fm-add-btn:hover { background: var(--border); color: var(--text); }

    /* ── Add-field dropdown ────────────────────────────────────────── */
    .fm-add-menu {
        position: absolute;
        top: calc(100% + 4px);
        right: 0;
        background: var(--bg);
        border: 1px solid var(--border);
        border-radius: 8px;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.14);
        padding: 4px;
        min-width: 190px;
        z-index: 200;
        display: flex;
        flex-direction: column;
    }
    .fm-add-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.75rem;
        width: 100%;
        background: none;
        border: none;
        padding: 0.28rem 0.55rem;
        cursor: pointer;
        border-radius: 5px;
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
        text-align: left;
    }
    .fm-add-item:hover { background: var(--border); }
    .fm-add-key {
        font-size: 0.78rem;
        font-weight: 600;
        color: var(--fy-key);
    }
    .fm-add-hint {
        font-size: 0.7rem;
        color: var(--muted);
        white-space: nowrap;
    }

    /* ── Overlay editor ────────────────────────────────────────────── */
    .fm-editor {
        display: grid;
        padding: 0;
        border-radius: 0 0 8px 8px;
        overflow: hidden;
    }
    .fm-editor > * {
        grid-area: 1 / 1;
        font-family: 'JetBrains Mono', 'Fira Code', monospace;
        font-size: 0.82rem;
        line-height: 1.6;
        padding: 0.55rem 0.75rem;
        margin: 0;
        white-space: pre-wrap;
        word-break: break-word;
        tab-size: 2;
    }

    .fm-hl {
        pointer-events: none;
        color: var(--text);
        overflow: hidden;
    }
    :global(.fy-key)     { color: var(--fy-key); }
    :global(.fy-str)     { color: var(--fy-str); }
    :global(.fy-bool)    { color: var(--fy-bool); font-style: italic; }
    :global(.fy-num)     { color: var(--fy-num); }
    :global(.fy-null)    { color: var(--fy-null); font-style: italic; }
    :global(.fy-comment) { color: var(--fy-comment); font-style: italic; }
    :global(.fy-p)       { color: var(--fy-p); }

    .fm-textarea {
        background: transparent;
        border: none;
        outline: none;
        resize: none;
        overflow: hidden;
        color: transparent;
        caret-color: var(--text);
        z-index: 1;
        min-height: 2.5rem;
    }
    .fm-textarea:not(.has-content) { color: var(--text); }
    .fm-textarea::placeholder { color: var(--muted); opacity: 0.45; }
</style>
