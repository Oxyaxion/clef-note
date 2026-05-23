/**
 * Reactive browsing history for wiki-style back/forward navigation.
 *
 * Usage:
 *   const nav = createNavigation();
 *   nav.push('My Note');
 *
 *   const target = nav.peekBack(); // name to navigate to, or null
 *   await selectNote(target!, false);
 *   nav.stepBack();               // commit after async navigation succeeds
 */

export interface Navigation {
	readonly canBack: boolean;
	readonly canForward: boolean;
	push(name: string): void;
	peekBack(): string | null;
	peekForward(): string | null;
	stepBack(): void;
	stepForward(): void;
}

export function createNavigation(): Navigation {
	let history = $state<string[]>([]);
	let index = $state(-1);

	return {
		get canBack() { return index > 0; },
		get canForward() { return index < history.length - 1; },

		push(name: string) {
			history = [...history.slice(0, index + 1), name];
			index = history.length - 1;
		},

		peekBack(): string | null {
			return index > 0 ? history[index - 1] : null;
		},

		peekForward(): string | null {
			return index < history.length - 1 ? history[index + 1] : null;
		},

		stepBack() { if (index > 0) index--; },
		stepForward() { if (index < history.length - 1) index++; },
	};
}
