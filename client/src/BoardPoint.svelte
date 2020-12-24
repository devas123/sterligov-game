<script lang="ts">
    import { getColorValue, isInCorner } from "./const";

    export let cx: number;
    export let cy: number;
    export let row: number;
    export let col: number;
    export let can_select: boolean;
    export let dot_base_radius: number;
    export let cone_base_radius: number;
    export let is_selected: boolean;
    export let is_cone: boolean;
    export let cone_color: number;
    export let should_highlight: boolean;

    let dot_radius = dot_base_radius;
    let cone_radius = cone_base_radius;
    const select_area_radius = cone_base_radius * 1.5;
    $: dot_radius =
        (is_selected ? dot_base_radius * 1.5 : dot_base_radius) -
        (isInCorner(row, col) ? 0 : 1);
    $: cone_radius = is_selected ? cone_base_radius * 1.5 : cone_base_radius;
</script>

<style>
    .big_point {
        fill: black;
        stroke-width: 1px;
        stroke: rgb(255, 255, 255);
    }
    .board-point:hover {
        opacity: 50%;
        cursor: pointer;
        stroke-width: 2px;
        stroke: aliceblue;
    }
    .my-cone:hover {
        cursor: pointer;
    }
    .selected {
        stroke-width: 2px;
    }

    .board-point.selected {
        stroke-width: 2px;
        stroke: black;
        fill: white;
    }

    .select-area {
        opacity: 0;
        fill: white;
    }
    .select-area:hover {
        cursor: pointer;
    }
</style>

{#if is_cone}
    <circle
        {cx}
        {cy}
        r={cone_radius}
        fill={getColorValue(cone_color)}
        stroke-width="1"
        stroke="black"
        class={is_cone ? 'cone' + (is_selected ? ' selected' : '') + (should_highlight ? ' my-cone' : '') : 'select-area'}
        on:click />
{:else}
    <circle
        {cx}
        {cy}
        r={dot_radius}
        class={(can_select ? 'board-point' : '') + (isInCorner(row, col) ? ' big_point' : '') + (is_selected ? ' selected' : '')}
           />
    <circle
        {cx}
        {cy}
        r={select_area_radius}
        class="select-area"
        on:click />
{/if}
