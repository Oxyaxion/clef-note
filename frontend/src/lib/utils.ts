export function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** True when a rejected promise is an aborted fetch (AbortController.abort()). */
export function isAbortError(e: unknown): boolean {
    return e instanceof DOMException && e.name === 'AbortError';
}

/**
 * True for hrefs safe to navigate to: web, mail, tel, anchors, relative paths.
 * Blocks executable schemes (javascript:, data:, vbscript:…). Used before
 * opening a link href that could originate from raw markdown in a note.
 */
export function isSafeHref(href: string): boolean {
    return /^(https?:\/\/|mailto:|tel:|\/|\.\/|#)/i.test(href.trim());
}

export function debounce<T extends unknown[]>(fn: (...args: T) => void, delay: number): (...args: T) => void {
    let timer: ReturnType<typeof setTimeout> | null = null;
    return (...args: T) => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => fn(...args), delay);
    };
}
