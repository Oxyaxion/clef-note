/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { build, version } from '$service-worker';

declare const self: ServiceWorkerGlobalScope;

const CACHE = `clef-note-${version}`;

// Cache only the content-hashed immutable chunks (_app/immutable/**).
// index.html, the SW script itself, and all API responses go directly to
// the network — caching them caused stale note lists and missed deployments.
const IMMUTABLE = build.filter(url => url.includes('/_app/immutable/'));

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE).then(cache => cache.addAll(IMMUTABLE)).then(() => self.skipWaiting())
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys()
			.then(async keys => {
				for (const key of keys) {
					if (key !== CACHE) await caches.delete(key);
				}
				await self.clients.claim();
			})
	);
});

self.addEventListener('fetch', (event) => {
	if (event.request.method !== 'GET') return;

	// Only serve from cache for content-hashed immutable assets.
	// Everything else (index.html, API calls, notes) goes to the network.
	const url = new URL(event.request.url);
	if (!url.pathname.startsWith('/_app/immutable/')) return;

	event.respondWith(
		caches.match(event.request).then(cached => cached ?? fetch(event.request))
	);
});
