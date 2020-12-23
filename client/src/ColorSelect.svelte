<script lang="ts">
    import { createEventDispatcher } from "svelte";

    import {
        BLUE,
        getColorString,
        GREEN,
        ORANGE,
        PURPLE,
        RED,
        YELLOW,
    } from "./const";

    let colors = [PURPLE, GREEN, ORANGE, YELLOW, RED, BLUE];
    export let players_colors: Map<number, number>;
    export let selected: number;
    const dispatch = createEventDispatcher();
    function selectColor() {
        if (selected) {
            const e = {
                color: selected,
            };
            dispatch("colorselected", e);
        }
    }

    const getFreeColors = (clrs: number[], plrs_clrs: Map<number, number>) => {
        if (!players_colors) {
            return clrs;
        }
        const colorsArray = Array.from(plrs_clrs.values());
        return clrs?.filter(c => !colorsArray.find(k => k === c));
    }

    let freeColors = [];

    $: freeColors = [...getFreeColors(colors, players_colors), selected];

</script>

<style>
</style>

<!-- svelte-ignore a11y-no-onchange -->
<select bind:value={selected} on:change={selectColor}>
    {#each freeColors as color}
        <option value={color}>{getColorString(color)}</option>
    {/each}
</select>
