<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Board from "./Board.svelte";
  import { getColorString, getColorValue } from "./const";
  import { userId, userToken } from "./stores";
  import { pop } from "svelte-spa-router";
  import type { Move, Player, RoomDesc } from "./model";
  import {
    createWebSocketForRoomRequest,
    gameStateRequest,
    getRoomPlayersRequest,
    makeAMoveRequest,
    roomResolveRequest,
    startGameRequest,
    validatePathRequest,
  } from "./functions";

  let players: Player[] = [];
  let cones = {};
  let selectedCones = [];
  let socket: WebSocket;
  let moves: Move[] = [];
  let next_player_to_move = 0;
  let my_color: number;
  let players_colors = new Map<number, number>();
  let room_state: RoomDesc;
  let tm;
  let itvl;
  let connected = false;

  export let params: { id: any };
  export let highlightedPath = [];

  async function validatePath(path: number[][]) {
    return validatePathRequest(path, $userToken, params.id);
  }

  function getPlayer(user_id: number): Player {
    return players.find((p) => p.user_id === user_id);
  }

  async function select(e: {
    detail: { row: number; col: number; isCone: boolean; coneIndex: number };
  }) {
    const { row, col, isCone, coneIndex } = e.detail;
    console.log(`Select cone ${row}, ${col}, ${coneIndex}, ${isCone}`);
    let newCones = selectedCones;
    if (isCone) {
      if (
        selectedCones &&
        selectedCones.length > 0 &&
        selectedCones[0][0] == row &&
        selectedCones[0][1] == col
      ) {
        newCones = [];
      } else {
        newCones = [[row, col]];
      }
    } else if (selectedCones && selectedCones.length > 0) {
      newCones = [...selectedCones, [row, col]];
    }
    console.log(
      `Select cone, ${row}, ${col}, path: ${JSON.stringify(
        selectedCones
      )}, new path: ${JSON.stringify(newCones)}`
    );
    if (newCones.length > 1) {
      const valid = await validatePath(newCones);
      if (valid) {
        selectedCones = newCones;
      }
    } else {
      selectedCones = newCones;
    }
  }

  async function makeAMove(path: number[][]) {
    await makeAMoveRequest(path, $userToken, params.id);
    selectedCones = [];
  }

  async function startGame() {
    return startGameRequest($userToken, params.id);
  }

  const roomRes = roomResolveRequest(params.id).then((r) => (room_state = r));

  function handleWsEvent(event) {
    if (event.data === "pong") {
      pong();
      return;
    }
    console.log("Message from server", event.data);
    const update = JSON.parse(event.data);
    const { name } = update;
    switch (name) {
      case "player_joined": {
        let { user_id, player_cones, player_name, player_color } = update;
        if (!players.find((p) => p.user_id === user_id)) {
          players = [
            ...players,
            { user_id, color: player_color, name: player_name },
          ];
        }

        const new_cones = { ...cones };
        for (const key in new_cones) {
          if (Object.prototype.hasOwnProperty.call(new_cones, key)) {
            const element = new_cones[key];
            if (element === player_color) {
              delete new_cones[key];
            }
          }
        }
        for (let index = 0; index < player_cones.length; index++) {
          new_cones[
            `${player_cones[index][0]},${player_cones[index][1]}`
          ] = player_color;
        }
        cones = new_cones;
        // console.log(cones);
        break;
      }
      case "player_left": {
        let { user_id, next_turn } = update;
        players = players.filter((p) => p.user_id !== user_id);
        next_player_to_move = next_turn;
        break;
      }
      case "move_made":
        let { by_user_id, path, next_player, game_finished } = update;
        const p = players.find((p) => p.user_id === by_user_id);
        moves = [...moves, { path, by: p }];
        const l = path.length;
        const m = cones[`${path[0][0]},${path[0][1]}`];
        delete cones[`${path[0][0]},${path[0][1]}`];
        let new_cones = { ...cones };
        new_cones[`${path[l - 1][0]},${path[l - 1][1]}`] = m;
        cones = new_cones;
        next_player_to_move = next_player % players.length;
        if (game_finished) {
          room_state = { ...room_state, game_finished, winner: by_user_id };
        }
        break;
      case "room_state_update":
        const { room: r } = update as { room: RoomDesc };
        room_state = { ...r };
    }
  }

  onDestroy(() => {
    if (tm) {
      clearTimeout(tm);
    }
    if (itvl) {
      clearInterval(itvl);
    }
    if (socket) {
      try {
        socket.close();
      } catch (err) {
        console.error(err);
      }
    }
  });

  function pong() {
    clearTimeout(tm);
  }

  const handleWsConnect = (socket: WebSocket) => async (event) => {
    console.log("Connected to server", event);
    clearTimeout(tm);
    clearInterval(itvl);
    itvl = setInterval(ping(socket), 10000);
    connected = true;
    await refreshRoomState().catch(console.error);
  };

  const recreateSocket = () => {
    if (socket) {
      socket.close();
    }
    connected = false;
    socket = createWebSocketForRoomRequest($userToken, params.id);
    socket.addEventListener("open", handleWsConnect(socket));
    // Listen for messages
    socket.addEventListener("message", handleWsEvent);
    socket.addEventListener("close", () => connected = false);
  };

  const ping = (socket: WebSocket) => () => {
    socket.send("ping");
    clearTimeout(tm);
    tm = setTimeout(recreateSocket, 5000);
  };

  onMount(async () => {
    socket = createWebSocketForRoomRequest($userToken, params.id);
    socket.addEventListener("open", handleWsConnect(socket));
    socket.addEventListener("message", handleWsEvent);
    socket.addEventListener("close", () => connected = false);
  });

  async function refreshRoomState() {
    const received = await getRoomPlayersRequest(params.id);
    players = [...received, ...players].filter(
      (value, index, arr) =>
        arr.findIndex((vv) => value.user_id === vv.user_id) === index
    );
    // console.log(`Players: ${JSON.stringify(players)}`);
    const gameStateRes = await gameStateRequest(params.id);
    const cones_res = gameStateRes || {};
    const { cones: cns, players_colors: plrs_clrs, moves: mvs } = cones_res;
    cones = { ...cones, ...cns };
    players_colors = new Map(
      Object.entries(plrs_clrs).map(([a, b]) => [+a, +b])
    );
    moves = [];
    mvs.forEach((m) => {
      const player = players.find((p) => p.color === m[0]);
      if (player) {
        moves.push(<Move>{ by: player, path: m[1] });
      }
    });
  }

  function getNextPlayer(players: any[], next_move: number) {
    return players[next_move];
  }

  async function handleKeydown(
    event: KeyboardEvent & { currentTarget: EventTarget & Window }
  ) {
    if (event.keyCode === 13 && selectedCones && selectedCones.length > 1) {
      await makeAMove(selectedCones);
    }
  }

  $: {
    if (players) {
      for (const player of players) {
        players_colors.set(player.user_id, player.color);
      }
      my_color = players_colors.get(+$userId);
    }
    // console.log(room)
  }
</script>

<style>
  .controls {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding-top: 1em;
  }

  .bold {
    font-weight: bold;
  }
  .moves-section {
    display: flex;
    flex-direction: column;
    overflow: auto;
    height: 10px;
    flex-grow: 1;
    margin-bottom: 1rem;
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

  .main-grid {
    position: relative;
    display: grid;
    grid-template-columns: 8fr 3fr;
    column-gap: 2px;
    align-content: stretch;
    justify-items: center;
    height: 100%;
    width: 100%;
  }
  .line {
    padding: 5px 0;
  }
</style>

<svelte:window on:keydown={async (event) => await handleKeydown(event)} />

{#await roomRes}
  <p>Loading room state</p>
{:then _}
  <div class="main-grid">
    <Board
      on:pointselected={async (e) => {
        await select(e);
      }}
      {my_color}
      {highlightedPath}
      {cones}
      {selectedCones}
      my_move={getNextPlayer(players, next_player_to_move)?.user_id == $userId} />
    <div class="controls">
      {#if !connected}
        <section>Connecting to server...</section>
      {/if}
      {#if +$userId == room_state.created_by}
        <!-- svelte-ignore empty-block -->
        {#if !room_state.game_started && !room_state.game_finished}
          <button on:click={startGame} disabled={!connected}>Start game</button>
        {:else if !room_state.game_started}{/if}
      {/if}
      {#if room_state.game_started && !room_state.game_finished}
        {#if getNextPlayer(players, next_player_to_move)?.user_id == $userId}
          <p>Your move.</p>
        {:else}
          <p>
            Waiting for
            {getNextPlayer(players, next_player_to_move)?.name}
            to make a move
          </p>
        {/if}
        <div class="line">
          Your color is
          <span
            style="background-color: {getColorValue(my_color)};">{getColorString(my_color)}</span>
        </div>
        <div>
          <button disabled={!connected} on:click={async () => await makeAMove(selectedCones)}>Make a
            move</button>
          <button on:click={() => (selectedCones = [])}>Clear</button>
        </div>
      {:else if room_state.game_finished}
        {#if +$userId === room_state.winner}
          <p>Game finished. You won!</p>
        {:else}
          <p>Game finished. Winner: {getPlayer(room_state.winner)?.name}</p>
        {/if}
      {:else}
        <p>Waiting for the game to start.</p>
      {/if}
      <div class="constant"><button on:click={() => pop()}>Leave</button></div>
      <div class="users">
        <p>Players:</p>
        <section class="users">
          <ul>
            {#each players as player, i}
              <li>
                <span
                  class={i === next_player_to_move ? 'bold' : ''}>{player.name}</span>
              </li>
            {/each}
          </ul>
        </section>
      </div>
      <div class="moves-section">
        <p>Moves:</p>
        {#each moves as move}
          <div
            style="color: {players_colors.get(move.by.user_id)};"
            class="path-row"
            on:mouseenter={() => (highlightedPath = move.path)}
            on:mouseleave={() => (highlightedPath = [])}>
            {move.by?.name + ': ' + JSON.stringify(move.path[0]) + '->' + JSON.stringify(move.path[move.path.length - 1])}
          </div>
        {/each}
      </div>
    </div>
  </div>
{:catch err}
  <p>Error loading room {JSON.stringify(err)}</p>
{/await}
