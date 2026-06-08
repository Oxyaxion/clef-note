/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { build, files, version } from '$service-worker';

declare const self: ServiceWorkerGlobalScope;

const CACHE = `clef-note-${version}`;
const ASSETS = [...build, ...files];

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE).then((cache) => cache.addAll(ASSETS)).then(() => self.skipWaiting())
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys().then(async (keys) => {
			for (const key of keys) {
				if (key !== CACHE) await caches.delete(key);
			}
			// Purge any dynamic-content entries that an older SW version may have
			// cached under the current key (e.g. /notes responses).
			const cache = await caches.open(CACHE);
			for (const req of await cache.keys()) {
				const p = new URL(req.url).pathname;
				if (p.startsWith('/notes') || p.startsWith('/api/') ||
					p.startsWith('/backlinks/') || p.startsWith('/auth')) {
					await cache.delete(req);
				}
			}
			await self.clients.claim();
			// Tell open tabs to reload so they pick up the new SW immediately
			// instead of needing a manual second refresh.
			for (const client of await self.clients.matchAll({ type: 'window' })) {
				client.postMessage({ type: 'SW_UPDATED' });
			}
		})
	);
});

self.addEventListener('fetch', (event) => {
	// Only intercept GET requests — PUT/POST/DELETE must reach the network directly.
	if (event.request.method !== 'GET') return;

	const url = event.request.url;
	// Dynamic content must never be served from cache: note list, note content,
	// API endpoints, backlinks, and auth. Serving stale note data from cache
	// causes newly-created notes to appear empty and missing from the sidebar.
	if (url.includes('/notes') ||
		url.includes('/api/') ||
		url.includes('/backlinks/') ||
		url.includes('/auth')) return;

	event.respondWith(
		(async () => {
			const cached = await caches.match(event.request);
			if (cached) return cached;
			const response = await fetch(event.request);
			if (response.status === 200) {
				const cache = await caches.open(CACHE);
				cache.put(event.request, response.clone());
			}
			return response;
		})()
	);
});
