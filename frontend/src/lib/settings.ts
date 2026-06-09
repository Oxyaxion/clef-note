export interface PartitionSettings {
    homePage?: string;
}

export interface AppSettings {
    fontFamily: string;
    fontSize: number;    // rem, e.g. 1.1
    lineHeight: number;  // e.g. 1.7
    customCss: string;
    partitions: Record<string, PartitionSettings>;
    dateFormat: string;
    mobileReadOnly: boolean;
}

export const FONT_PRESETS = [
    { id: 'inter',  label: 'Inter',    value: "'Inter', system-ui, sans-serif" },
    { id: 'system', label: 'System',   value: "ui-sans-serif, system-ui, -apple-system, sans-serif" },
    { id: 'serif',  label: 'Serif',    value: "Georgia, 'Times New Roman', serif" },
    { id: 'mono',   label: 'Mono',     value: "'JetBrains Mono', 'Fira Code', ui-monospace, monospace" },
];

export const DATE_FORMATS = [
    { id: 'long-en', label: 'Full',     example: 'Wednesday, December 25, 2025' },
    { id: 'eu',      label: 'European', example: '25/12/2025' },
    { id: 'iso',     label: 'ISO',      example: '2025-12-25' },
    { id: 'us',      label: 'American', example: '12/25/2025' },
];

export const DEFAULT: AppSettings = {
    fontFamily: "'Inter', system-ui, sans-serif",
    fontSize: 1.1,
    lineHeight: 1.7,
    customCss: '',
    partitions: {},
    dateFormat: 'long-en',
    mobileReadOnly: false,
};

export function applySettings(s: AppSettings): void {
    const root = document.documentElement;
    root.style.setProperty('--ui-font', s.fontFamily);
    root.style.setProperty('--ui-font-size', `${s.fontSize}rem`);
    root.style.setProperty('--ui-line-height', String(s.lineHeight));

    let styleEl = document.getElementById('clef-user-css') as HTMLStyleElement | null;
    if (!styleEl) {
        styleEl = document.createElement('style');
        styleEl.id = 'clef-user-css';
        document.head.appendChild(styleEl);
    }
    styleEl.textContent = s.customCss;
}
