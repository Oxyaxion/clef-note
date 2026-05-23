import type { Frontmatter } from './api';

function yamlScalar(v: unknown): string {
	if (typeof v !== 'string') return String(v);
	if (/[:#\[\]{}&*!|>'"%@`,]/.test(v) || v.trim() !== v || v === '') {
		return `"${v.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
	}
	return v;
}

/** Serialize a frontmatter object back to a YAML block (with --- delimiters). */
export function serializeFrontmatter(fm: Frontmatter): string {
	const entries = Object.entries(fm).filter(([, v]) => {
		if (v === null || v === undefined || v === '') return false;
		if (Array.isArray(v) && v.length === 0) return false;
		return true;
	});
	if (entries.length === 0) return '';

	const lines = entries.map(([key, value]) => {
		if (Array.isArray(value)) {
			return `${key}:\n${(value as unknown[]).map(v => `  - ${yamlScalar(v)}`).join('\n')}`;
		}
		return `${key}: ${yamlScalar(value)}`;
	});

	return `---\n${lines.join('\n')}\n---\n\n`;
}
