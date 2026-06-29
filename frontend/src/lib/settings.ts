import type { ThemeId } from './theme';
import { applyTheme } from './theme';

export interface PartitionSettings {
    theme?: ThemeId;
    fontFamily?: string;
    fontSize?: number;
    lineHeight?: number;
    homePage?: string;
    dateFormat?: string;
}

export interface AppSettings {
    customCss: string;
    mobileReadOnly: boolean;
    /** Slug of the partition to open on app load. Empty = use the server's active one. */
    defaultPartition: string;
    partitions: Record<string, PartitionSettings>;
}

export const FONT_PRESETS = [
    { id: 'inter',  label: 'Inter',  value: "'Inter', system-ui, sans-serif" },
    { id: 'system', label: 'System', value: "ui-sans-serif, system-ui, -apple-system, sans-serif" },
    { id: 'serif',  label: 'Serif',  value: "Georgia, 'Times New Roman', serif" },
    { id: 'mono',   label: 'Mono',   value: "'JetBrains Mono', 'Fira Code', ui-monospace, monospace" },
];

export const DATE_FORMATS = [
    { id: 'long-en', label: 'Full',     example: 'Wednesday, December 25, 2025' },
    { id: 'eu',      label: 'European', example: '25/12/2025' },
    { id: 'iso',     label: 'ISO',      example: '2025-12-25' },
    { id: 'us',      label: 'American', example: '12/25/2025' },
];

export const PARTITION_DEFAULTS: Required<PartitionSettings> = {
    theme: 'default',
    fontFamily: "'Inter', system-ui, sans-serif",
    fontSize: 1.1,
    lineHeight: 1.7,
    homePage: '',
    dateFormat: 'long-en',
};

export const DEFAULT: AppSettings = {
    customCss: '',
    mobileReadOnly: false,
    defaultPartition: '',
    partitions: {},
};

export function activePartitionSettings(s: AppSettings, slug: string): PartitionSettings {
    return s.partitions[slug] ?? {};
}

export function applySettings(s: AppSettings, ps: PartitionSettings): void {
    applyTheme((ps.theme ?? PARTITION_DEFAULTS.theme) as ThemeId);
    const root = document.documentElement;
    root.style.setProperty('--ui-font', ps.fontFamily ?? PARTITION_DEFAULTS.fontFamily);
    root.style.setProperty('--ui-font-size', `${ps.fontSize ?? PARTITION_DEFAULTS.fontSize}rem`);
    root.style.setProperty('--ui-line-height', String(ps.lineHeight ?? PARTITION_DEFAULTS.lineHeight));

    let styleEl = document.getElementById('clef-user-css') as HTMLStyleElement | null;
    if (!styleEl) {
        styleEl = document.createElement('style');
        styleEl.id = 'clef-user-css';
        document.head.appendChild(styleEl);
    }
    styleEl.textContent = s.customCss;
}

/** Migrate old flat settings.json format (v1) to per-partition structure (v2). */
export function migrateSettings(raw: Record<string, unknown>, slugs: string[]): AppSettings {
    const hasOldFields = 'fontFamily' in raw || 'fontSize' in raw || 'lineHeight' in raw || 'dateFormat' in raw;
    const partitions: Record<string, PartitionSettings> = (raw.partitions as Record<string, PartitionSettings>) ?? {};

    if (hasOldFields) {
        for (const slug of slugs) {
            const existing = partitions[slug] ?? {};
            partitions[slug] = {
                fontFamily: raw.fontFamily as string | undefined,
                fontSize: raw.fontSize as number | undefined,
                lineHeight: raw.lineHeight as number | undefined,
                dateFormat: raw.dateFormat as string | undefined,
                ...existing,
            };
        }
    }

    // Migrate legacy homePages map
    const homePages = raw.homePages as Record<string, string> | undefined;
    if (homePages) {
        for (const [slug, homePage] of Object.entries(homePages)) {
            partitions[slug] = { ...partitions[slug], homePage: partitions[slug]?.homePage ?? homePage };
        }
    }

    return {
        customCss: (raw.customCss as string) ?? '',
        mobileReadOnly: (raw.mobileReadOnly as boolean) ?? false,
        defaultPartition: (raw.defaultPartition as string) ?? '',
        partitions,
    };
}
