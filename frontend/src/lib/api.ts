import { emit } from './events';
import type { AppSettings } from './settings';
import { storage } from './storage';

const BASE = import.meta.env.VITE_API_BASE ?? '';

export const session = storage.token;

function authHeaders(extra?: Record<string, string>): Record<string, string> {
    const h: Record<string, string> = { ...extra };
    const token = storage.token.get();
    if (token) h['Authorization'] = `Bearer ${token}`;
    return h;
}

async function apiFetch(url: string, init?: RequestInit): Promise<Response> {
    const res = await fetch(url, init);
    if (res.status === 401) {
        session.clear();
        emit(window, 'auth:expired');
    }
    return res;
}

function encodeName(name: string): string {
    return name.split('/').map(encodeURIComponent).join('/');
}

export interface NoteMeta {
    name: string;
    pinned?: boolean;
    is_template?: boolean;
    is_index?: boolean;
}

export interface Frontmatter {
    title?: string;
    tags?: string[];
    date?: string;
    status?: string;
    aliases?: string[];
    type?: string;
    due?: string;
    url?: string;
    author?: string;
    rating?: number;
    pinned?: boolean;
    locked?: boolean;
    area?: string;
    priority?: string;
    project?: string;
    toc?: boolean;
    [key: string]: unknown;
}

export interface NoteContent {
    name: string;
    /** Body without YAML frontmatter — ready for TipTap */
    content: string;
    /** Parsed frontmatter fields */
    frontmatter: Frontmatter;
}

export interface BacklinksResponse {
    note: string;
    backlinks: string[];
}

export interface NoteQueryResult {
    name: string;
    title?: string;
    date?: string;
    status?: string;
    tags: string[];
    aliases?: string[];
    note_type?: string;
    due?: string;
    url?: string;
    author?: string;
    rating?: number;
    pinned?: boolean;
    area?: string;
    priority?: string;
    project?: string;
}

export async function listNotes(): Promise<NoteMeta[]> {
    const res = await apiFetch(`${BASE}/notes`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list notes');
    return res.json();
}

export async function getNote(name: string): Promise<NoteContent> {
    const res = await apiFetch(`${BASE}/notes/${encodeName(name)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error(`Note not found: ${name}`);
    return res.json();
}

export async function saveNote(name: string, content: string): Promise<void> {
    const res = await apiFetch(`${BASE}/notes/${encodeName(name)}`, {
        method: 'PUT',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ content }),
    });
    if (!res.ok) throw new Error('Failed to save note');
}

export async function renameNote(oldName: string, newName: string): Promise<void> {
    const res = await apiFetch(`${BASE}/notes/${encodeName(oldName)}`, {
        method: 'PATCH',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ new_name: newName }),
    });
    if (res.status === 409) throw new Error('A note with that name already exists');
    if (!res.ok) throw new Error('Rename failed');
}

export async function getBacklinks(name: string): Promise<BacklinksResponse> {
    const res = await apiFetch(`${BASE}/backlinks/${encodeName(name)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch backlinks');
    return res.json();
}

export async function uploadAsset(file: File): Promise<string> {
    const form = new FormData();
    form.append('file', file);
    const res = await apiFetch(`${BASE}/assets`, { method: 'POST', headers: authHeaders(), body: form });
    if (!res.ok) throw new Error('Failed to upload asset');
    const data = await res.json();
    return `${BASE}${data.url}`;
}

export interface TagCount {
    tag: string;
    count: number;
}

export async function getFieldValues(field: string): Promise<string[]> {
    const res = await apiFetch(`${BASE}/api/field-values?field=${encodeURIComponent(field)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch field values');
    return res.json();
}

export async function listTags(): Promise<TagCount[]> {
    const res = await apiFetch(`${BASE}/api/tags`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list tags');
    return res.json();
}

export async function queryNotes(q: string): Promise<NoteQueryResult[]> {
    const res = await apiFetch(`${BASE}/api/query?q=${encodeURIComponent(q)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to query notes');
    return res.json();
}

export async function deleteNote(name: string): Promise<void> {
    const res = await apiFetch(`${BASE}/notes/${encodeName(name)}`, { method: 'DELETE', headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to delete note');
}

export interface SearchResult {
    name: string;
    title?: string;
    snippet: string;
}

export async function searchContent(q: string): Promise<SearchResult[]> {
    const res = await apiFetch(`${BASE}/api/search?q=${encodeURIComponent(q)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to search');
    return res.json();
}

export async function getAliases(): Promise<Record<string, string>> {
    const res = await apiFetch(`${BASE}/api/aliases`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch aliases');
    return res.json();
}

// ── Assets ───────────────────────────────────────────────────────────────────

export interface AssetMeta {
    name: string;
    size: number;
}

export async function listAssets(): Promise<AssetMeta[]> {
    const res = await apiFetch(`${BASE}/api/assets`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list assets');
    return res.json();
}

export async function deleteAsset(name: string): Promise<void> {
    const res = await apiFetch(`${BASE}/assets/${encodeURIComponent(name)}`, {
        method: 'DELETE',
        headers: authHeaders(),
    });
    if (!res.ok) throw new Error('Failed to delete asset');
}

export interface MediaUsage {
    used_assets: string[];
    used_drawings: string[];
}

export async function getMediaUsage(): Promise<MediaUsage> {
    const res = await apiFetch(`${BASE}/api/media-usage`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch media usage');
    return res.json();
}

// ── Drawings ──────────────────────────────────────────────────────────────────

export interface ExcalidrawData {
    elements: unknown[];
    appState: Record<string, unknown>;
    files: Record<string, unknown>;
}

export async function listDrawings(): Promise<string[]> {
    const res = await apiFetch(`${BASE}/api/drawings`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list drawings');
    return res.json();
}

export async function getDrawing(name: string): Promise<ExcalidrawData> {
    const res = await apiFetch(`${BASE}/api/drawings/${encodeName(name)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Drawing not found');
    return res.json();
}

export async function saveDrawing(name: string, data: ExcalidrawData): Promise<void> {
    const res = await apiFetch(`${BASE}/api/drawings/${encodeName(name)}`, {
        method: 'PUT',
        headers: { ...authHeaders(), 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error('Failed to save drawing');
}

export async function deleteDrawing(name: string): Promise<void> {
    const res = await apiFetch(`${BASE}/api/drawings/${encodeName(name)}`, {
        method: 'DELETE',
        headers: authHeaders(),
    });
    if (!res.ok) throw new Error('Failed to delete drawing');
}

export async function getDrawingPreview(name: string): Promise<string> {
    const res = await apiFetch(`${BASE}/api/drawing-preview/${encodeName(name)}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('No preview');
    return res.text();
}

export async function saveDrawingPreview(name: string, svg: string): Promise<void> {
    await apiFetch(`${BASE}/api/drawing-preview/${encodeName(name)}`, {
        method: 'PUT',
        headers: { ...authHeaders(), 'Content-Type': 'image/svg+xml' },
        body: svg,
    });
}

// ── Auth ──────────────────────────────────────────────────────────────────────

export async function login(password: string): Promise<void> {
    const res = await fetch(`${BASE}/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
    });
    if (res.status === 401) throw new Error('Wrong password');
    if (!res.ok) throw new Error('Login failed');
    const data = await res.json();
    storage.token.set(data.token);
}

export async function logout(): Promise<void> {
    await apiFetch(`${BASE}/auth/logout`, { method: 'POST', headers: authHeaders() });
    storage.token.clear();
}

// ── Key management ────────────────────────────────────────────────────────────

export interface KeysResponse {
    api_key: string;
}

/** Public — no auth needed. Called at app startup to bootstrap the auth key. */
export async function fetchKeys(): Promise<KeysResponse> {
    const res = await apiFetch(`${BASE}/api/key`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch keys');
    return res.json();
}


// ── App settings ─────────────────────────────────────────────────────────────

export async function getSettings(): Promise<Record<string, unknown>> {
    const res = await apiFetch(`${BASE}/api/settings`, { headers: authHeaders() });
    if (!res.ok) return {};
    return res.json();
}

export async function putSettings(s: AppSettings): Promise<void> {
    await apiFetch(`${BASE}/api/settings`, {
        method: 'PUT',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify(s),
    });
}

// ── Frontmatter helpers ───────────────────────────────────────────────────────

export { serializeFrontmatter } from './frontmatter';
