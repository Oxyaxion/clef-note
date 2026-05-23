import type { NoteMeta } from './api';

export type TreeNode =
	| { kind: 'note';   path: string; label: string; pinned: boolean; is_index: boolean }
	| { kind: 'folder'; path: string; label: string; children: TreeNode[] };

export type DisplayItem =
	| { kind: 'note';   path: string; label: string; depth: number; pinned: boolean; is_index: boolean }
	| { kind: 'folder'; path: string; label: string; depth: number; open: boolean };

function sortLevel(nodes: TreeNode[]): TreeNode[] {
	const pinned   = nodes.filter(n => n.kind === 'note'   && n.pinned    && !n.is_index);
	const folders  = nodes.filter(n => n.kind === 'folder');
	const unpinned = nodes.filter(n => n.kind === 'note'   && !n.pinned   && !n.is_index);
	for (const f of folders) {
		if (f.kind === 'folder') f.children = sortLevel(f.children);
	}
	return [...pinned, ...folders, ...unpinned];
}

export function buildTree(noteList: NoteMeta[]): TreeNode[] {
	const root: TreeNode[] = [];
	for (const note of noteList) {
		const parts = note.name.split('/');
		let current = root;
		for (let i = 0; i < parts.length; i++) {
			const part = parts[i];
			if (i === parts.length - 1) {
				current.push({ kind: 'note', path: note.name, label: part, pinned: note.pinned ?? false, is_index: note.is_index ?? false });
			} else {
				let folder = current.find(
					(n): n is Extract<TreeNode, { kind: 'folder' }> => n.kind === 'folder' && n.label === part
				);
				if (!folder) {
					folder = { kind: 'folder', path: parts.slice(0, i + 1).join('/'), label: part, children: [] };
					current.push(folder);
				}
				current = folder.children;
			}
		}
	}
	return sortLevel(root);
}

export function flatten(nodes: TreeNode[], openFolders: Set<string>, depth = 0): DisplayItem[] {
	const result: DisplayItem[] = [];
	for (const node of nodes) {
		if (node.kind === 'note') {
			result.push({ kind: 'note', path: node.path, label: node.label, depth, pinned: node.pinned, is_index: node.is_index });
		} else {
			const open = openFolders.has(node.path);
			result.push({ kind: 'folder', path: node.path, label: node.label, depth, open });
			if (open) result.push(...flatten(node.children, openFolders, depth + 1));
		}
	}
	return result;
}
