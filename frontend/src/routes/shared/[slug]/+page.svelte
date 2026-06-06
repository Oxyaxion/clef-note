<script lang="ts">
	import { page } from '$app/stores';
	import SharedNote from '$lib/SharedNote.svelte';

	const slug = $derived($page.params.slug ?? '');
</script>

<SharedNote {slug} />
