import tippy from 'tippy.js';

const MOBILE_MQ = '(max-width: 640px)';

/**
 * Lifecycle handle for a suggestion menu popup (slash commands, wiki-links…).
 *
 * Two strategies, chosen at creation time by viewport width:
 *  - Desktop: floating popup anchored to the cursor (tippy.js), as before.
 *  - Mobile: the menu is docked to the bottom of the *visual* viewport — i.e.
 *    just above the on-screen keyboard — full width. This avoids the keyboard
 *    covering it and keeps it put while the note scrolls underneath.
 */
export interface SuggestionPopup {
	/** Update the cursor reference rect (floating mode only; no-op when docked). */
	setRect(getRect: () => DOMRect): void;
	/** Temporarily hide the menu (e.g. on Escape). */
	hide(): void;
	/** Tear down the popup and any listeners. */
	destroy(): void;
}

export function createSuggestionPopup(
	menuEl: HTMLElement,
	getRect: () => DOMRect,
): SuggestionPopup {
	return window.matchMedia(MOBILE_MQ).matches
		? dockedPopup(menuEl)
		: floatingPopup(menuEl, getRect);
}

// ── Desktop: floating near the cursor ──────────────────────────────────────
function floatingPopup(menuEl: HTMLElement, getRect: () => DOMRect): SuggestionPopup {
	let rectFn = getRect;
	const [inst] = tippy('body', {
		getReferenceClientRect: () => rectFn(),
		appendTo: () => document.body,
		content: menuEl,
		showOnCreate: true,
		interactive: true,
		trigger: 'manual',
		placement: 'bottom-start',
	});

	return {
		setRect(fn) {
			rectFn = fn;
			inst?.setProps({ getReferenceClientRect: () => rectFn() });
		},
		hide() {
			inst?.hide();
		},
		destroy() {
			if (inst && !inst.state.isDestroyed) inst.destroy();
		},
	};
}

// ── Mobile: docked above the keyboard ───────────────────────────────────────
function dockedPopup(menuEl: HTMLElement): SuggestionPopup {
	menuEl.classList.add('slash-menu--docked');
	document.body.appendChild(menuEl);

	const vv = window.visualViewport;

	const reposition = () => {
		if (vv) {
			// The keyboard occupies the gap between the visual viewport and the
			// layout viewport's bottom edge. Sit the menu right on top of it.
			const keyboardInset = Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
			menuEl.style.bottom = `${keyboardInset}px`;
			// Never exceed half of the space actually visible above the keyboard.
			menuEl.style.maxHeight = `${Math.round(vv.height * 0.5)}px`;
		} else {
			menuEl.style.bottom = '0px';
		}
	};

	reposition();
	vv?.addEventListener('resize', reposition);
	vv?.addEventListener('scroll', reposition);
	window.addEventListener('resize', reposition);

	return {
		setRect() {
			// Docked mode is anchored to the keyboard, not the cursor — nothing to do.
		},
		hide() {
			menuEl.style.display = 'none';
		},
		destroy() {
			vv?.removeEventListener('resize', reposition);
			vv?.removeEventListener('scroll', reposition);
			window.removeEventListener('resize', reposition);
			menuEl.remove();
		},
	};
}
