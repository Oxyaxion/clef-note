import { emit } from './events';
import type { NoteQueryResult, TagCount } from './api';
import { PRINT_FIELDS, getFieldText, type PrintSpec } from './queryBlockHelpers';

export function makeLink(text: string, row: NoteQueryResult): HTMLButtonElement {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'query-result-name';
    btn.textContent = text;
    btn.addEventListener('click', () => {
        emit(document, 'wiki-navigate', row.name);
    });
    return btn;
}

export function appendResultRows(container: HTMLElement, rows: NoteQueryResult[], print: PrintSpec | null): void {
    if (!rows.length) {
        const empty = document.createElement('span');
        empty.className = 'query-empty';
        empty.textContent = 'No results';
        container.appendChild(empty);
        return;
    }
    rows.forEach((row) => {
        const item = document.createElement('div');
        item.className = 'query-result-item';

        if (print) {
            let hasLink = false;
            print.fields.forEach((field) => {
                const isLink = field === 'name' || field === 'path' || field === 'title';
                if (isLink) {
                    const text = field === 'title' ? (row.title || row.name)
                        : field === 'path' ? row.name
                        : (row.name.split('/').pop() ?? row.name);
                    item.appendChild(makeLink(text, row));
                    hasLink = true;
                } else if (PRINT_FIELDS.has(field)) {
                    const text = getFieldText(field, row);
                    if (text) {
                        const span = document.createElement('span');
                        span.className = 'query-result-meta';
                        span.textContent = text;
                        item.appendChild(span);
                    }
                }
            });
            if (!hasLink) {
                const btn = makeLink(getFieldText(print.fields[0], row) || row.name, row);
                btn.className = 'query-result-name';
                item.prepend(btn);
            }
        } else {
            item.appendChild(makeLink(row.title || row.name, row));
            if (row.title && row.title !== row.name) {
                const pathEl = document.createElement('span');
                pathEl.className = 'query-result-path';
                pathEl.textContent = row.name;
                item.appendChild(pathEl);
            }
            const metaParts: string[] = [];
            if (row.tags.length) metaParts.push(row.tags.join(', '));
            if (row.date) metaParts.push(row.date);
            if (row.status) metaParts.push(row.status);
            if (metaParts.length) {
                const meta = document.createElement('span');
                meta.className = 'query-result-meta';
                meta.textContent = metaParts.join(' · ');
                item.appendChild(meta);
            }
        }

        container.appendChild(item);
    });
}

export function buildTagCloud(
    tags: TagCount[],
    onTagClick: (tag: string) => void,
    activeIndex = -1,
    isFieldCloud = false,
    labelPrefix = '#',
): HTMLDivElement {
    const cloud = document.createElement('div');
    cloud.className = 'query-tag-cloud' + (isFieldCloud ? ' query-field-cloud' : '');
    tags.forEach(({ tag }, i) => {
        const chip = document.createElement('button');
        chip.className = 'query-tag-chip' + (i === activeIndex ? ' active' : '');
        const label = document.createElement('span');
        label.className = 'query-tag-name';
        label.textContent = labelPrefix === '#' ? `#${tag}` : tag;
        chip.appendChild(label);
        chip.addEventListener('mousedown', (e) => {
            e.preventDefault();
            onTagClick(tag);
        });
        cloud.appendChild(chip);
    });
    return cloud;
}

export function buildTagCountCloud(
    tags: TagCount[],
    onTagClick: (tag: string) => void,
): HTMLDivElement {
    const cloud = document.createElement('div');
    cloud.className = 'query-tag-cloud';
    tags.forEach(({ tag, count }) => {
        const chip = document.createElement('button');
        chip.className = 'query-tag-chip';
        const tagNameEl = document.createElement('span');
        tagNameEl.className = 'query-tag-name';
        tagNameEl.textContent = tag;
        const tagCountEl = document.createElement('span');
        tagCountEl.className = 'query-tag-count';
        tagCountEl.textContent = String(count);
        chip.append(tagNameEl, tagCountEl);
        chip.addEventListener('click', () => onTagClick(tag));
        cloud.appendChild(chip);
    });
    return cloud;
}
