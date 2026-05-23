<script lang="ts">
	interface Props {
		isIndex?: boolean;
	}
	let { isIndex = false }: Props = $props();
</script>

{#if isIndex}
	<svg viewBox="0 0 12 12" fill="none" aria-hidden="true">
		<rect x="1" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
		<rect x="7" y="1" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
		<rect x="1" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
		<rect x="7" y="7" width="4" height="4" rx="0.5" stroke="currentColor" stroke-width="1.2"/>
	</svg>
{:else}
	<svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
		<path d="M3 2h7l3 3v9H3V2z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
		<path d="M10 2v3h3" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>
	</svg>
{/if}
