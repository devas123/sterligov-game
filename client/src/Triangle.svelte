<script lang="ts">
    export let a: number[];
    export let b: number[];
    export let c: number[];
    export let cls: string;
</script>

<style>
    .purple {
        fill: blueviolet;
    }
    .blue {
        fill: hsl(216, 100%, 50%);
    }
    .red {
        fill: red;
    }
    .yellow {
        fill: yellow;
    }
    .orange {
        fill: orange;
    }
    .green {
        fill: greenyellow;
    }
    .opaque {
        opacity: 20%;
        fill: white;
        stroke: black;
        stroke-width: 1px;
    }
</style>

<polygon
    points="{a[0]},{a[1]} 
                           {b[0]},{b[1]} 
                           {c[0]},{c[1]}"
    class={cls} />
