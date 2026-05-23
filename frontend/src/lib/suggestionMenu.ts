export interface SuggestionMenu<T> {
    el: HTMLDivElement;
    update(items: T[]): void;
    move(delta: number): void;
    select(): T | null;
}

export function buildSuggestionMenu<T>(
    renderItem: (item: T, btn: HTMLButtonElement) => void,
    onSelect: (item: T) => void,
    emptyText = 'No results',
): SuggestionMenu<T> {
    let selectedIndex = 0;
    let items: T[] = [];

    const el = document.createElement('div');
    el.className = 'slash-menu';

    function render() {
        el.innerHTML = '';
        if (items.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'slash-menu-empty';
            empty.textContent = emptyText;
            el.appendChild(empty);
            return;
        }
        items.forEach((item, i) => {
            const btn = document.createElement('button');
            btn.className = 'slash-menu-item' + (i === selectedIndex ? ' selected' : '');
            renderItem(item, btn);
            btn.addEventListener('mousedown', (e) => {
                e.preventDefault();
                onSelect(item);
            });
            el.appendChild(btn);
        });
    }

    return {
        el,
        update(newItems: T[]) {
            items = newItems;
            selectedIndex = 0;
            render();
        },
        move(delta: number) {
            if (!items.length) return;
            selectedIndex = (selectedIndex + delta + items.length) % items.length;
            render();
            el.querySelectorAll<HTMLElement>('.slash-menu-item')[selectedIndex]
                ?.scrollIntoView({ block: 'nearest' });
        },
        select(): T | null {
            return items[selectedIndex] ?? null;
        },
    };
}
