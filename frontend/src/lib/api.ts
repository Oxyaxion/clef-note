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

export async function getNote(name: string, signal?: AbortSignal): Promise<NoteContent> {
    const res = await apiFetch(`${BASE}/notes/${encodeName(name)}`, { headers: authHeaders(), signal });
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

export async function getBacklinks(name: string, signal?: AbortSignal): Promise<BacklinksResponse> {
    const res = await apiFetch(`${BASE}/backlinks/${encodeName(name)}`, { headers: authHeaders(), signal });
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

export async function listTags(signal?: AbortSignal): Promise<TagCount[]> {
    const res = await apiFetch(`${BASE}/api/tags`, { headers: authHeaders(), signal });
    if (!res.ok) throw new Error('Failed to list tags');
    return res.json();
}

export async function queryNotes(q: string, signal?: AbortSignal): Promise<NoteQueryResult[]> {
    const res = await apiFetch(`${BASE}/api/query?q=${encodeURIComponent(q)}`, { headers: authHeaders(), signal });
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

export async function searchContent(q: string, signal?: AbortSignal): Promise<SearchResult[]> {
    const res = await apiFetch(`${BASE}/api/search?q=${encodeURIComponent(q)}`, { headers: authHeaders(), signal });
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

export interface NoteStub {
    name: string;
    title: string | null;
    body_len: number;
}

export async function listStubs(maxBytes = 500): Promise<NoteStub[]> {
    const res = await apiFetch(`${BASE}/api/notes/stubs?max_bytes=${maxBytes}`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch stubs');
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

export async function getDrawingPreview(name: string, signal?: AbortSignal): Promise<string> {
    const res = await apiFetch(`${BASE}/api/drawing-preview/${encodeName(name)}`, { headers: authHeaders(), signal });
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

export interface AuthConfig {
    oidc_enabled: boolean;
    provider_name: string | null;
    password_disabled: boolean;
}

export async function getAuthConfig(): Promise<AuthConfig> {
    try {
        const res = await fetch(`${BASE}/api/auth/config`);
        if (!res.ok) return { oidc_enabled: false, provider_name: null, password_disabled: false };
        return res.json();
    } catch {
        return { oidc_enabled: false, provider_name: null, password_disabled: false };
    }
}

export async function exchangeOidcCode(code: string): Promise<void> {
    const res = await fetch(`${BASE}/auth/oidc/exchange`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code }),
    });
    if (!res.ok) throw new Error('OIDC exchange failed');
    const data = await res.json();
    storage.token.set(data.token);
}

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

// ── Partitions ────────────────────────────────────────────────────────────────

export interface PartitionInfo {
    slug: string;
    name: string;
    active: boolean;
    has_sync: boolean;
}

export async function listPartitions(): Promise<PartitionInfo[]> {
    const res = await apiFetch(`${BASE}/api/partitions`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list partitions');
    return res.json();
}

export async function switchPartition(slug: string): Promise<void> {
    const res = await apiFetch(`${BASE}/api/partitions/active`, {
        method: 'POST',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ slug }),
    });
    if (!res.ok) throw new Error('Failed to switch partition');
}

export async function createPartition(name: string): Promise<PartitionInfo> {
    const res = await apiFetch(`${BASE}/api/partitions`, {
        method: 'POST',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ name }),
    });
    if (res.status === 409) throw new Error('A partition with that name already exists');
    if (!res.ok) throw new Error('Failed to create partition');
    return res.json();
}

export async function renamePartition(slug: string, name: string): Promise<void> {
    const res = await apiFetch(`${BASE}/api/partitions/${encodeURIComponent(slug)}`, {
        method: 'PATCH',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ name }),
    });
    if (!res.ok) throw new Error('Failed to rename partition');
}

export async function deletePartition(slug: string): Promise<void> {
    const res = await apiFetch(`${BASE}/api/partitions/${encodeURIComponent(slug)}`, {
        method: 'DELETE',
        headers: authHeaders(),
    });
    if (res.status === 409) throw new Error('Cannot delete the active partition');
    if (!res.ok) throw new Error('Failed to delete partition');
}

// ── Git sync ──────────────────────────────────────────────────────────────────

export interface SyncStatus {
    configured: boolean;
    last_sync_at: string | null;
    last_error: string | null;
}

export async function getSyncStatus(): Promise<SyncStatus> {
    const res = await apiFetch(`${BASE}/api/sync/status`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to fetch sync status');
    return res.json();
}

export async function triggerSync(): Promise<void> {
    await apiFetch(`${BASE}/api/sync`, {
        method: 'POST',
        headers: authHeaders(),
    });
}

// ── Shares ────────────────────────────────────────────────────────────────────

export interface ShareMeta {
    slug: string;
    note: string;
    created_at: string;
    expires_at: string | null;
    has_password: boolean;
}

export interface SharedNoteContent {
    slug: string;
    title: string;
    content: string;
    note: string;
    expires_at: string | null;
}

export async function listShares(): Promise<ShareMeta[]> {
    const res = await apiFetch(`${BASE}/api/shares`, { headers: authHeaders() });
    if (!res.ok) throw new Error('Failed to list shares');
    return res.json();
}

export async function createShare(opts: {
    slug: string;
    note: string;
    expires_at: string | null;
    password?: string;
}): Promise<ShareMeta> {
    const res = await apiFetch(`${BASE}/api/shares`, {
        method: 'POST',
        headers: { ...authHeaders(), 'Content-Type': 'application/json' },
        body: JSON.stringify(opts),
    });
    if (res.status === 409) throw new Error('slug-conflict');
    if (!res.ok) throw new Error('Failed to create share');
    return res.json();
}

export async function deleteShare(slug: string): Promise<void> {
    const res = await apiFetch(`${BASE}/api/shares/${encodeURIComponent(slug)}`, {
        method: 'DELETE',
        headers: authHeaders(),
    });
    if (!res.ok) throw new Error('Failed to delete share');
}

export async function updateShare(slug: string, opts: {
    expires_at?: string | null;
    password?: string;
}): Promise<ShareMeta> {
    const res = await apiFetch(`${BASE}/api/shares/${encodeURIComponent(slug)}`, {
        method: 'PATCH',
        headers: { ...authHeaders(), 'Content-Type': 'application/json' },
        body: JSON.stringify(opts),
    });
    if (!res.ok) throw new Error('Failed to update share');
    return res.json();
}

/** Fetch a public shared note. Pass password via X-Share-Password header if needed. */
export async function getSharedNote(
    slug: string,
    password?: string,
): Promise<{ ok: true; data: SharedNoteContent } | { ok: false; status: 401 | 404 | 410 | number }> {
    const headers: Record<string, string> = {};
    if (password) headers['X-Share-Password'] = password;
    const res = await fetch(`${BASE}/api/shared/${encodeURIComponent(slug)}`, { headers });
    if (res.ok) return { ok: true, data: await res.json() };
    return { ok: false, status: res.status };
}

// ── Frontmatter helpers ───────────────────────────────────────────────────────

export { serializeFrontmatter } from './frontmatter';
