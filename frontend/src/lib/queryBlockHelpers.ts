import type { NoteQueryResult } from './api';

// ── Today keyword resolution ──────────────────────────────────────────────────

/** Replace `today` (in value position, after `:`) with the current ISO date. */
export function resolveToday(q: string): string {
    const today = new Date().toISOString().slice(0, 10);
    return q.replace(/(:\s*)today\b/gi, `$1${today}`);
}

// ── Autocomplete token detection ──────────────────────────────────────────────

export interface ActiveToken {
    type: 'tag' | 'field';
    field: string;
    partial: string;
    start: number;
    end: number;
}

export const COMPLETABLE_FIELDS = new Set(['status', 'area', 'author', 'type', 'due', 'rating', 'tag']);

export function getActiveToken(el: HTMLInputElement): ActiveToken | null {
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
        // Strip trailing comparison operator (<=, >=, <, >) before looking up the field.
        const rawField = tok.slice(0, ci).toLowerCase();
        const field = rawField.replace(/[<>=]+$/, '');
        if (COMPLETABLE_FIELDS.has(field)) {
            return { type: 'field', field, partial: tok.slice(ci + 1), start: s, end: e };
        }
    }
    return null;
}

// ── Print directive ───────────────────────────────────────────────────────────

export interface PrintSpec {
    fields: string[];
}

const PRINT_FIELDS_PAT = '(?:name|path|title|tags|date|status|area|author|due|rating|url)';
const PRINT_RE = new RegExp(`\\bprint\\s+(${PRINT_FIELDS_PAT}(?:[\\s,]+${PRINT_FIELDS_PAT})*)\\s*$`, 'i');

export function parsePrint(q: string): { cleanQuery: string; print: PrintSpec | null } {
    const m = q.match(PRINT_RE);
    if (!m) return { cleanQuery: q, print: null };
    const fields = m[1].split(/[\s,]+/).filter(Boolean).map(f => f.toLowerCase());
    return {
        cleanQuery: q.slice(0, m.index).trim(),
        print: fields.length ? { fields } : null,
    };
}

export const PRINT_FIELDS = new Set(['name', 'path', 'title', 'tags', 'date', 'status', 'area', 'author', 'due', 'rating', 'url']);

export function getFieldText(field: string, row: NoteQueryResult): string {
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

export const ICON_LOCKED = `<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="7" width="10" height="8" rx="1.5"/><path d="M5 7V5a3 3 0 0 1 6 0v2"/></svg>`;
export const ICON_UNLOCKED = `<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="7" width="10" height="8" rx="1.5"/><path d="M5 7V5a3 3 0 0 1 5.83 1"/></svg>`;
