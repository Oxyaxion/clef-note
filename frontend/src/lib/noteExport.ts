/**
 * Pure note export/share utilities.
 * Each function returns a Promise so the caller can close the palette after it resolves.
 */

export function stripQueryBlocks(md: string): string {
	return md.replace(/^```query\n[\s\S]*?^```$/gm, '').replace(/\n{3,}/g, '\n\n').trim();
}

export async function copyMarkdown(markdown: string): Promise<void> {
	await navigator.clipboard.writeText(markdown);
}

export async function copyMarkdownClean(markdown: string): Promise<void> {
	await navigator.clipboard.writeText(stripQueryBlocks(markdown));
}

export async function copyHtml(markdown: string): Promise<void> {
	const html = document.querySelector('.ProseMirror')?.innerHTML ?? '';
	await navigator.clipboard.write([
		new ClipboardItem({
			'text/html':  new Blob([html],     { type: 'text/html' }),
			'text/plain': new Blob([markdown], { type: 'text/plain' }),
		}),
	]);
}

function triggerDownload(filename: string, content: string, mimeType: string): void {
	const blob = new Blob([content], { type: mimeType });
	const a = document.createElement('a');
	a.href = URL.createObjectURL(blob);
	a.download = filename;
	a.click();
	URL.revokeObjectURL(a.href);
}

export function downloadMd(noteName: string, markdown: string): void {
	const filename = (noteName.split('/').pop() ?? 'note') + '.md';
	triggerDownload(filename, markdown, 'text/markdown');
}

export function downloadMdClean(noteName: string, markdown: string): void {
	const filename = (noteName.split('/').pop() ?? 'note') + '.md';
	triggerDownload(filename, stripQueryBlocks(markdown), 'text/markdown');
}

export function printNote(): void {
	// Small delay so the palette DOM is removed before the print dialog opens.
	setTimeout(() => window.print(), 80);
}

export async function shareNote(noteName: string, markdown: string): Promise<void> {
	const name = noteName.split('/').pop() ?? 'note';
	await navigator.share({ title: name, text: markdown });
}

export const canShare = typeof navigator !== 'undefined' && 'share' in navigator;
