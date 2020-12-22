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
import { userId } from "./stores";

    let colors = [PURPLE, GREEN, ORANGE, YELLOW, RED, BLUE];
    export let players_colors: Map<number, number>
    let selected: number;
    const dispatch = createEventDispatcher();
    function selectColor() {
        if (selected) {
            const e = {
                color: selected,
            };
            dispatch("colorselected", e);
            selected = undefined;
        }
    }

    const getFreeColors = (clrs: number[], plrs_clrs: Map<number, number>) => {
        if (!players_colors) {
            return clrs;
        }
        const colorsArray = Array.from(plrs_clrs.values());
        return clrs?.filter(c => !colorsArray.find(k => k === c));
    }

    $: selected = players_colors.get(+$userId);
</script>

<style>
</style>

<!-- svelte-ignore a11y-no-onchange -->
<select bind:value={selected} on:change={selectColor}>
    {#each [selected, ...getFreeColors(colors, players_colors)] as color}
        <option value={color} selected={color === selected}>{getColorString(color)}</option>
    {/each}
</select>
