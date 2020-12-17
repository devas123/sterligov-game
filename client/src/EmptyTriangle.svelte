<script lang="ts">
    export let a: number[];
    export let b: number[];
    export let c: number[];
    export let cls;
</script>

<style>
    .simple {
        stroke: black;
        stroke-width: 1px;
        opacity: 20%;
    }
</style>

<line x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} class="simple" />
<line x1={b[0]} y1={b[1]} x2={c[0]} y2={c[1]} class="simple" />
<line x1={a[0]} y1={a[1]} x2={c[0]} y2={c[1]} class="simple" />
