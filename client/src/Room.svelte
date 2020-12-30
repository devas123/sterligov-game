<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Board from "./Board.svelte";
  import { getColorString, getColorValue, NEUTRAL } from "./const";
  import { userId, userToken } from "./stores";
  import { pop } from "svelte-spa-router";
  import type { Move, Player, RoomDesc } from "./model";
  import {
    chatMessageRequest,
    colorChangeRequest,
    createWebSocketForRoomRequest,
    gameStateRequest,
    getRoomPlayersRequest,
    leaveRoomRequest,
    makeAMoveRequest,
    roomResolveRequest,
    startGameRequest,
    validatePathRequest,
  } from "./functions";
  import Moves from "./Moves.svelte";
  import Chat from "./Chat.svelte";
  import Tabs from "./tabs/Tabs.svelte";
  import TabList from "./tabs/TabList.svelte";
  import Tab from "./tabs/Tab.svelte";
  import TabPanel from "./tabs/TabPanel.svelte";
  import ColorSelect from "./ColorSelect.svelte";

  let players: Player[] = [];
  let cones = {};
  let selectedCones = [];
  let socket: EventSource;
  let moves: Move[] = [];
  let next_player_to_move = 0;
  let my_color: number;
  let players_colors = new Map<number, number>();
  let room_state: RoomDesc;
  let connected = false;
  let chatMessages = [];
  let chatFocused = false;

  export let params: { id: any };
  export let highlightedPath = [];

  const onFocus = () => (chatFocused = true);
  const onBlur = () => (chatFocused = false);

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

  const roomRes = () => roomResolveRequest(params.id).then((r) => (room_state = r));

  function handleWsEvent(event) {
    if (event.data === "test") {
      return;
    }
    const update = JSON.parse(event.data);
    console.log("Data: ", update);
    const { name } = update;
    switch (name) {
      case "chat_message": {
        chatMessages = [...chatMessages, update];
        break;
      }
      case "player_joined": {
        let { user_id, player_cones, player_name, player_color } = update;
        if (!players.find((p) => p.user_id === user_id)) {
          players = [
            ...players,
            { user_id, color: player_color, name: player_name },
          ];
          players_colors = players_colors.set(+user_id, player_color)
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
        let { user_id, next_turn, remove_cones, player_color } = update;
        players = players.filter((p) => p.user_id !== user_id);
        next_player_to_move = next_turn;
        const new_pc = new Map<number,number>();
        players_colors.forEach((v, k) => { if (k != user_id) { new_pc.set(k, v) } });
        players_colors = new_pc;
        if (remove_cones && player_color != NEUTRAL) {
           const new_cones = {...cones};
           for (const key in new_cones) {
             if (Object.prototype.hasOwnProperty.call(new_cones, key)) {
               const color = new_cones[key];
               if (color == player_color) {
                 delete new_cones[key];
               }
             }
           }
           cones = new_cones;
        }
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
        room_state = r;
        break;
      case "game_state":
        const { cones: c, players_colors: pc } = update.game;
        cones = c;
        players_colors = new Map(Object.entries(pc).map(([a, b]) => [+a, +b]));
        break;
    }
  }

  onDestroy(() => {
    if (socket) {
      try {
        socket.close();
      } catch (err) {
        console.error(err);
      }
    }
  });

  const leaveRoom = () => {
    leaveRoomRequest($userToken, params.id);
    pop();
  };

  const handleWsConnect = async (event) => {
    console.log("Connected to server", event);
    connected = true;
    await refreshRoomState().catch(console.error);
  };

  const recreateSocket = () => {
    if (socket) {
      socket.close();
    }
    connected = false;
    socket = createWebSocketForRoomRequest($userToken, params.id);
    socket.addEventListener("open", handleWsConnect);
    socket.addEventListener("test", console.log);
    socket.addEventListener("error", (e: any) => {
      console.error("Error in sse:", e);
      if (e.readyState != EventSource.OPEN) {
        connected = false;
      }
    });
    socket.onmessage = handleWsEvent;
  };

  onMount(async () => {
    recreateSocket();
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

  const sendChatMessage = async (e) => {
    if (socket) {
      const { message } = e.detail;
      if (message) {
        await chatMessageRequest(message, $userToken, params.id);
      }
    }
  };

  async function handleKeydown(
    event: KeyboardEvent & { currentTarget: EventTarget & Window }
  ) {
    if (
      !chatFocused &&
      event.code === "Space" &&
      selectedCones &&
      selectedCones.length > 1
    ) {
      await makeAMove(selectedCones);
    }
  }

  const selectColor = async ({ color }: { color: number }) => {
    await colorChangeRequest($userToken, params.id, color);
  };

  $: {
    if (players) {
      my_color = players_colors.get(+$userId);
    }
    // console.log(room)
  }
</script>

<style>
  .controls {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-height: 0;
    min-width: 0;
    padding-bottom: 1em;
    padding-top: 1em;
  }

  .bold {
    font-weight: bold;
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
  .users {
    padding: 1rem 0;
  }
</style>

<svelte:window on:keydown={async (event) => await handleKeydown(event)} />

{#await roomRes()}
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
      game_started={room_state.game_started}
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
          <h3>Your move!</h3>
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
          <button
            disabled={!connected}
            on:click={async () => await makeAMove(selectedCones)}>Make a move</button>
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
        <p>You can select color</p>
        <ColorSelect
          selected={my_color}
          on:colorselected={(e) => selectColor(e.detail)}
          {players_colors} />
      {/if}
      <div class="constant"><button on:click={leaveRoom}>Leave</button></div>
      <section>
        <div>Players:</div>
        <section class="users">
            {#each players as player, i}
              <div>
                <span style="color: {getColorValue(players_colors.get(player.user_id))}">&#9679;</span>
                <span
                  class={i === next_player_to_move ? 'bold' : ''}>{player.name}</span>
                </div>
            {/each}
          </section>
      </section>
      <Tabs>
        <TabList>
          <Tab>Chat</Tab>
          <Tab>Moves</Tab>
        </TabList>
        <TabPanel>
          <Chat
            on:blur={onBlur}
            on:focus={onFocus}
            messages={chatMessages}
            {players_colors}
            on:messagesent={(e) => sendChatMessage(e)} />
        </TabPanel>
        <TabPanel>
          <Moves
            {moves}
            {players_colors}
            on:moveselected={(e) => (highlightedPath = e.detail)} />
        </TabPanel>
      </Tabs>
    </div>
  </div>
{:catch err}
  <p>Error loading room {JSON.stringify(err)}</p>
{/await}
