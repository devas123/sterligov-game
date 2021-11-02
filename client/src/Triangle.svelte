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
        fill: #a23131;
    }
    .yellow {
        fill: #b2b200;
    }
    .orange {
        fill: #e39821;
    }
    .green {
        fill: #6c9629;
    }
    .opaque {
        opacity: 50%;
        fill: white;
        stroke: #262626;
        stroke-width: 1px;
    }
</style>

<polygon
    points="{a[0]},{a[1]} 
                           {b[0]},{b[1]} 
                           {c[0]},{c[1]}"
    class={cls} />
