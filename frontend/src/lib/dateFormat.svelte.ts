/**
 * App-wide reactive date format setting.
 *
 * Stored here (not in slashCommands.ts) so any module can read or update
 * it without depending on the slash-command extension.
 */

export type DateFormatId = 'long-en' | 'eu' | 'iso' | 'us';

let current = $state<DateFormatId>('long-en');

export function getDateFormat(): DateFormatId { return current; }
export function setDateFormat(fmt: DateFormatId | string) { current = fmt as DateFormatId; }
