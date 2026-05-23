const KEYS = {
    token: 'clef_token',
    theme: 'clef-theme',
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
} as const;
