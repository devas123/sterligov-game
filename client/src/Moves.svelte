<script lang="ts">
    import { createEventDispatcher } from "svelte";
    export let players_colors;
    export let moves = [];
    const dispatch = createEventDispatcher();

    const selectPath = (path: []) => {
        dispatch("moveselected", path);
    };
</script>

<style>
    .moves-section {
        display: flex;
        flex-direction: column;
        overflow: auto;
        flex-grow: 0;
        margin-bottom: 1rem;
        width: 100%;
        height: 100%;
        min-height: 0;
    }
    .path-row {
        cursor: pointer;
        font-family: "Courier New", Courier, monospace;
        border-radius: 2px;
        margin-bottom: 2px;
    }
    .path-row:hover {
        background-color: rgb(66, 66, 14);
    }
</style>

{#if !!players_colors && !!moves}
    <div class="moves-section">
        {#each [...moves].reverse() as move}
            <div
                style="color: {players_colors.get(move.by.user_id)};"
                class="path-row"
                on:mouseenter={() => selectPath(move.path)}
                on:mouseleave={() => selectPath([])}>
                {move.by?.name + ': ' + JSON.stringify(move.path[0]) + '->' + JSON.stringify(move.path[move.path.length - 1])}
            </div>
        {/each}
    </div>
{/if}
