<script lang="ts">
  import {
    pointCounts,
    isInCorner,
    getColorString,
    small_triangles_coordinates,
    small_triangles_center,
  } from "./const";
  import { createEventDispatcher } from "svelte";
  import Triangle from "./Triangle.svelte";
import EmptyTriangle from "./EmptyTriangle.svelte";
  const dispatch = createEventDispatcher();

  function select(row: number, col: number) {
    const my_cone = getConeColorNumber(cones, row, col);
    if (my_color === my_cone || (!my_cone && my_move)) {
      const e = {
        row,
        col,
        isCone: isCone(cones, row, col),
        coneIndex: find_index(selectedCones, row, col),
      };
      dispatch("pointselected", e);
    }
  }

  let width = 500;
  let height = (width * Math.sqrt(5)) / 2;
  export let my_color: number;
  export let my_move: boolean;
  export let cones = {};
  export let selectedCones = [];
  export let highlightedPath = [];
  let dot_radius = 3;
  let cone_radius = 8;
  function getXCoordinate(col: number, colSize: number, width: number) {
    let minStep = width / 16;
    return col * minStep + (width - minStep * colSize) / 2 + width / 12;
  }
  function getYCoordinate(row: number, height: number) {
    return row * (height / 21) + (10 * Math.sqrt(5)) / 2;
  }
  function canSelectCone(
    cones: { [x: string]: number },
    path: number[][],
    row: number,
    col: number
  ) {
    if (isCone(cones, row, col)) {
      return true;
    }
    return path && path.length > 0;
  }
  const find_index = (path: number[][], row: number, col: number) => {
    // console.log(`looking for ${row}, ${col}`);
    for (let index = 0; index < path.length; index++) {
      const element = path[index];
      if (element[0] === row && element[1] === col) {
        return index;
      }
    }
    return -1;
  };
  function isCone(cones: { [x: string]: number }, row: number, col: number) {
    return cones.hasOwnProperty(`${row},${col}`);
  }

  function isSelected(path: number[][], row: number, col: number) {
    return find_index(path, row, col) >= 0;
  }

  function getConeColor(
    cones: { [x: string]: number },
    row: number,
    col: number
  ) {
    return getColorString(getConeColorNumber(cones, row, col));
  }

  function getConeColorNumber(
    cones: { [x: string]: number },
    row: number,
    col: number
  ) {
    return cones[`${row},${col}`];
  }

  const triangle = (component: any, a: number[], b: number[], c: number[], color: string) => {
    return {
      component,
      props: {
        a: [
          getXCoordinate(a[1], pointCounts[a[0]], width),
          getYCoordinate(a[0], height),
        ],
        b: [
          getXCoordinate(b[1], pointCounts[b[0]], width),
          getYCoordinate(b[0], height),
        ],
        c: [
          getXCoordinate(c[1], pointCounts[c[0]], width),
          getYCoordinate(c[0], height),
        ],
        cls: `triangle ${color}`,
      },
    };
  };

  const big_triangles = [
    triangle(Triangle, [0, 0], [5, 5], [5, 10], `purple`),
    triangle(Triangle, [5, 5], [5, 0], [10, 0], `blue`),
    triangle(Triangle, [10, 0], [15, 0], [15, 5], `red`),
    triangle(Triangle, [15, 5], [15, 10], [20, 0], `yellow`),
    triangle(Triangle, [15, 10], [15, 15], [10, 10], `orange`),
    triangle(Triangle, [10, 10], [5, 15], [5, 10], `green`),
  ];
  const small_triangles = small_triangles_coordinates.map((arr) =>
    triangle(Triangle, arr[0], arr[1], arr[2], "opaque")
  );
  const center_triangles = small_triangles_center.map((arr) =>
    triangle(EmptyTriangle, arr[0], arr[1], arr[2], "simple")
  );
</script>

<style>
  .main-svg {
    min-height: 400px;
    min-width: 400px;
    max-width: 800px;
    max-height: 100%;
    width: auto;
  }
  .big_circle {
    fill: rgb(175, 175, 175);
    stroke-width: 6px;
    stroke: black;
  }

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
    opacity: 50%;
    cursor: pointer;
  }
  .selected {
    opacity: 50%;
    fill: brown;
    stroke-width: 3px;
  }

  .select-area {
    opacity: 0;
    fill: white;
  }
  .select-area:hover {
    opacity: 10%;
    cursor: pointer;
  }
</style>

<svg class="main-svg" viewBox="0 0 {height} {height}">
  <circle
    cx={getXCoordinate(5, pointCounts[10], width)}
    cy={getYCoordinate(10, height)}
    r={height / 2 - 10}
    class="big_circle" />
  {#each big_triangles as bt}
    <svelte:component this={bt.component} {...bt.props} />
  {/each}
  {#each small_triangles as st}
    <svelte:component this={st.component} {...st.props} />
  {/each}
  {#each center_triangles as st}
    <svelte:component this={st.component} {...st.props} />
  {/each}

  {#each pointCounts as p, i}
    {#each Array(p) as _, point}
      <circle
        cx={getXCoordinate(point, p, width)}
        cy={getYCoordinate(i, height)}
        r={isInCorner(i, point) ? dot_radius : dot_radius - 1}
        class={(canSelectCone(cones, selectedCones, i, point) ? 'board-point' : '') + (isInCorner(i, point) ? ' big_point' : '') + (isSelected(selectedCones, i, point) ? ' selected' : '')} />
        <circle
          cx={getXCoordinate(point, p, width)}
          cy={getYCoordinate(i, height)}
          r={cone_radius}
          fill={getConeColor(cones, i, point)}
          stroke-width="1"
          stroke="black"
          class={(isCone(cones, i, point) ? ('cone' + (isSelected(selectedCones, i, point) ? ' selected' : '') + (my_color == getConeColorNumber(cones, i, point) && my_move ? ' my-cone' : '')) : 'select-area')}
          on:click={(_e) => select(i, point)} />
    {/each}
  {/each}
  {#each highlightedPath as pathPoint}
    <circle
      cx={getXCoordinate(pathPoint[1], pointCounts[pathPoint[0]], width)}
      cy={getYCoordinate(pathPoint[0], height)}
      r="6"
      fill="pink"
      stroke-width="1"
      stroke="black"
      opacity="50%"
      class="cone" />
  {/each}
</svg>
