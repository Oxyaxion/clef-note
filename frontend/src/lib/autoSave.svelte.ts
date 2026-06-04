import { saveNote, serializeFrontmatter, type Frontmatter } from './api';

export function createAutoSave(onSuccess: (name: string, fm: Frontmatter) => void) {
	let timer: ReturnType<typeof setTimeout> | null = null;
	let saving = $state(false);
	let saveFailed = $state(false);
	// The edit waiting to be persisted. Kept so a pending save can be flushed
	// (e.g. on note switch) instead of silently dropped.
	let pending: { name: string; fm: Frontmatter; body: string } | null = null;

	async function save(name: string, fm: Frontmatter, body: string) {
		saving = true;
		saveFailed = false;
		try {
			await saveNote(name, serializeFrontmatter(fm) + body);
			onSuccess(name, fm);
		} catch {
			saveFailed = true;
		} finally {
			saving = false;
		}
	}

	function cancel() {
		if (timer) { clearTimeout(timer); timer = null; }
		pending = null;
	}

	function schedule(name: string, fm: Frontmatter, body: string) {
		if (timer) clearTimeout(timer);
		pending = { name, fm, body };
		timer = setTimeout(() => {
			timer = null;
			const p = pending;
			pending = null;
			if (p) save(p.name, p.fm, p.body);
		}, 800);
	}

	/** Persist any pending edit immediately (fire-and-forget). Call before switching notes. */
	function flush() {
		if (timer) { clearTimeout(timer); timer = null; }
		const p = pending;
		pending = null;
		if (p) save(p.name, p.fm, p.body);
	}

	return {
		get saving() { return saving; },
		get saveFailed() { return saveFailed; },
		schedule,
		cancel,
		flush,
	};
}
