export function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export function debounce<T extends unknown[]>(fn: (...args: T) => void, delay: number): (...args: T) => void {
    let timer: ReturnType<typeof setTimeout> | null = null;
    return (...args: T) => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => fn(...args), delay);
    };
}
