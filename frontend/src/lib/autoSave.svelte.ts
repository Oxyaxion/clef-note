import { saveNote, serializeFrontmatter, type Frontmatter } from './api';

export function createAutoSave(onSuccess: (name: string, fm: Frontmatter) => void) {
	let timer: ReturnType<typeof setTimeout> | null = null;
	let saving = $state(false);
	let saveFailed = $state(false);

	function cancel() {
		if (timer) { clearTimeout(timer); timer = null; }
	}

	function schedule(name: string, fm: Frontmatter, body: string) {
		cancel();
		timer = setTimeout(async () => {
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
		}, 800);
	}

	return {
		get saving() { return saving; },
		get saveFailed() { return saveFailed; },
		schedule,
		cancel,
	};
}
