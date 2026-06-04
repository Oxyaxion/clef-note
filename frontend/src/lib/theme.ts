import { storage } from './storage';

export const THEMES = [
    { id: 'default',     label: 'Default' },
    { id: 'rose-pine',   label: 'Rosé Pine' },
    { id: 'dracula',     label: 'Dracula' },
    { id: 'solarized',   label: 'Solarized' },
    { id: 'catppuccin',  label: 'Catppuccin' },
    { id: 'github-dark', label: 'GitHub Dark' },
    { id: 'desert', label: 'Desert' },
    { id: 'gruvbox-light', label: 'Gruvbox Light' },
] as const;

export type ThemeId = typeof THEMES[number]['id'];

export function loadTheme(): ThemeId {
    return (storage.theme.get() as ThemeId) ?? 'default';
}

export function applyTheme(id: ThemeId) {
    if (id === 'default') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', id);
    }
    storage.theme.set(id);
}
