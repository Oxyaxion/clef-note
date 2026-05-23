/**
 * Typed wrappers around CustomEvent for all cross-component events in the app.
 * Use `emit` to dispatch and `on` to subscribe; both are type-safe.
 */

export interface AppEvents {
	'auth:expired':  void;
	'wiki-navigate': string;
	'notes:changed': void;
	'insert-image':  string;
	'link-prompt':   { x: number; y: number; currentUrl: string; selectedText: string };
}

type Detail<K extends keyof AppEvents> =
	AppEvents[K] extends void ? never : AppEvents[K];

type Handler<K extends keyof AppEvents> =
	AppEvents[K] extends void ? () => void : (detail: AppEvents[K]) => void;

/**
 * Dispatch a typed custom event on any EventTarget.
 * Pass `init` to set `bubbles`, `cancelable`, etc.
 */
export function emit<K extends keyof AppEvents>(
	target: EventTarget,
	type: K,
	detail?: Detail<K>,
	init?: EventInit,
): void {
	target.dispatchEvent(new CustomEvent(type, { detail, ...init }));
}

/**
 * Subscribe to a typed custom event. Returns an unsubscribe function.
 */
export function on<K extends keyof AppEvents>(
	target: EventTarget,
	type: K,
	handler: Handler<K>,
): () => void {
	const listener = (e: Event) =>
		(handler as (d: unknown) => void)((e as CustomEvent).detail);
	target.addEventListener(type, listener);
	return () => target.removeEventListener(type, listener);
}
