import { saveNote, serializeFrontmatter, type Frontmatter } from './api';

// A pending save is either a structured edit (frontmatter + body, from the rich
// editor) or a raw full-document edit (from the source view, saved verbatim).
type Pending =
	| { name: string; content: string; fm: Frontmatter }
	| { name: string; content: string; fm: null };

export function createAutoSave(onSuccess: (name: string, fm: Frontmatter) => void) {
	let timer: ReturnType<typeof setTimeout> | null = null;
	let saving = $state(false);
	let saveFailed = $state(false);
	// The edit waiting to be persisted. Kept so a pending save can be flushed
	// (e.g. on note switch) instead of silently dropped.
	let pending: Pending | null = null;

	async function persist(p: Pending) {
		saving = true;
		saveFailed = false;
		try {
			await saveNote(p.name, p.content);
			// Raw saves (fm === null) don't carry parsed metadata; the caller
			// re-syncs by refetching the note when leaving the source view.
			if (p.fm) onSuccess(p.name, p.fm);
		} catch {
			saveFailed = true;
		} finally {
			saving = false;
		}
	}

	function arm(p: Pending) {
		if (timer) clearTimeout(timer);
		pending = p;
		timer = setTimeout(() => {
			timer = null;
			const p = pending;
			pending = null;
			if (p) persist(p);
		}, 800);
	}

	function cancel() {
		if (timer) { clearTimeout(timer); timer = null; }
		pending = null;
	}

	/** Schedule a structured save from the rich editor (frontmatter + body). */
	function schedule(name: string, fm: Frontmatter, body: string) {
		arm({ name, content: serializeFrontmatter(fm) + body, fm });
	}

	/** Schedule a raw full-document save from the source view, stored verbatim. */
	function scheduleRaw(name: string, content: string) {
		arm({ name, content, fm: null });
	}

	/** Persist any pending edit immediately. Returns once the write resolves. */
	function flush(): Promise<void> {
		if (timer) { clearTimeout(timer); timer = null; }
		const p = pending;
		pending = null;
		return p ? persist(p) : Promise.resolve();
	}

	return {
		get saving() { return saving; },
		get saveFailed() { return saveFailed; },
		schedule,
		scheduleRaw,
		cancel,
		flush,
	};
}
