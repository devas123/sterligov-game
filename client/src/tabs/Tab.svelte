<script>
	import { getContext } from 'svelte';
	import { TABS } from './Tabs.svelte'
	const tab = {};
	const { registerTab, selectTab, selectedTab } = getContext(TABS);
	registerTab(tab);
</script>

<style>
	button {
		background: none;
		border: none;
		border-radius: 0;
		margin: 0;
		color: rgb(32, 19, 19);
	}
	
	.selected {
		border-bottom: 2px solid rgb(232, 255, 255);
		color: #333;
	}
</style>

<button class:selected="{$selectedTab === tab}" on:click="{() => selectTab(tab)}">
	<slot></slot>
</button>
