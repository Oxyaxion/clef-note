const KEYS = {
    token: 'clef_token',
    theme: 'clef-theme',
    sidebarCollapsed: 'clef-sidebar-collapsed',
    sidebarWidth: 'clef-sidebar-width',
} as const;

type StorageKey = typeof KEYS[keyof typeof KEYS];

function get(key: StorageKey): string | null {
    return localStorage.getItem(key);
}

function set(key: StorageKey, value: string): void {
    localStorage.setItem(key, value);
}

function remove(key: StorageKey): void {
    localStorage.removeItem(key);
}

export const storage = {
    token: {
        get: (): string => get(KEYS.token) ?? '',
        set: (t: string) => set(KEYS.token, t),
        clear: () => remove(KEYS.token),
        exists: (): boolean => !!get(KEYS.token),
    },
    theme: {
        get: (): string | null => get(KEYS.theme),
        set: (id: string) => set(KEYS.theme, id),
    },
    sidebar: {
        getCollapsed: (): boolean => get(KEYS.sidebarCollapsed) === 'true',
        setCollapsed: (v: boolean) => set(KEYS.sidebarCollapsed, String(v)),
        getWidth: (defaultWidth: number): number =>
            parseInt(get(KEYS.sidebarWidth) ?? '', 10) || defaultWidth,
        setWidth: (w: number) => set(KEYS.sidebarWidth, String(w)),
    },
} as const;
