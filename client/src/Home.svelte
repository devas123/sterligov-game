<script lang="ts">
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { createdAt, userId, userName, userToken } from "./stores";
  import type { RoomDesc } from "./model";
  import {
    addUserRequest,
clearToken,
        createRoomRequest,
    getRoomsListRequest,
processToken,
        refreshTokenRequest,
  } from "./functions";

  let roomName;
  let allRooms: RoomDesc[] = [];

  const create_room = async (room_name: string, usrToken) => {
    const roomResponse = await createRoomRequest(room_name, usrToken);
    allRooms = !!allRooms.find((r) => r.id === roomResponse.id)
      ? allRooms
      : [...allRooms, roomResponse];
    push(`/room/${roomResponse.room.id}`);
  };

  async function addUser(name: string) {
    if (name && name.length > 0 && name.length <= 15) {
      const response = await addUserRequest(name);
      // console.log(response);
      userToken.set(response.token);
      userId.set(response.user_id);
      localStorage.setItem("x-user-token", response.token);
      localStorage.setItem("x-user-id", response.user_id);
      localStorage.setItem("x-user-name", name);
      localStorage.setItem("x-user-id-created-at", response.created_at);
    }
  }

  async function refresh(token: string) {
    return refreshTokenRequest(token)
      .catch((err) => {
        clearToken()
      })
      .then((t) => {
        if (t) {
          processToken(t);
          // console.log("Refreshed user token successfully", t);
        } else {
          clearToken();
        }
        return Promise.resolve();
      });
  }

  function logout() {
    userToken.set(null);
    userName.set("");
    localStorage.removeItem("x-user-token");
    localStorage.removeItem("x-user-id");
    localStorage.removeItem("x-user-name");
    localStorage.removeItem("x-user-id-created-at");
  }

  async function getRoomsList(): Promise<RoomDesc[]> {
    return getRoomsListRequest();
  }

  async function getRoomsAndUpdate() {
    const rooms = await getRoomsList();
    // console.log("Rooms: ", rooms);
    allRooms = rooms;
    return rooms;
  }

  onMount(async () => {
    getRoomsAndUpdate();

    if (!!$userToken) {
      await refresh($userToken);
    } else {
      logout();
    }
  });
</script>

<style>
  .controls {
    display: block;
    width: 100%;
    height: 100%;
  }
  .rooms-list {
    display: flex;
    flex-direction: column;
  }
  .create-room-form {
    display: flex;
    flex-direction: row;
  }

  .room-row {
    display: flex;
    flex-direction: row;
    width: 100%;
    max-width: 300px;
    cursor: pointer;
    justify-content: space-between;
  }
  .rooms-list * {
    margin-right: 1em;
  }

  .room-row:hover {
    opacity: 70%;
  }

  .fill {
    flex-grow: 1;
  }
  button {
    margin-left: 3px;
  }

  .add-user-form {
    margin-top: 1em;
    display: flex;
    flex-direction: row;
  }
</style>

<div class="controls">
  <h2>Welcome to chinese-checkers game!</h2>
  {#if !$userToken}
    <div class="add-user-form">
      <label for="name">
        Please input your name
        <input
          id="name"
          bind:value={$userName}
          placeholder="Name"
          type="text" />
      </label>
      <button
        on:click={async () => await addUser($userName)}
        disabled={!$userName || $userName.length === 0 || $userName.length > 15}>Go!</button>
    </div>
  {:else}
    <p>Your name is: {$userName}</p>
    <button on:click={logout}>Logout</button>
    <div class="create-room-form">
      <label for="room-name">
        Input room name
        <input
          id="room-name"
          bind:value={roomName}
          placeholder="Room name"
          type="text" />
      </label>
      <button
        on:click={async () => await create_room(roomName, $userToken)}
        disabled={!roomName || roomName.length === 0 || roomName.length > 15}>Create
        room</button>
      <button
        on:click={async () => {
          await getRoomsAndUpdate();
        }}>Refresh rooms list</button>
    </div>
    <p>ROOMS:</p>
    <div class="rooms-list">
      {#each allRooms as room}
        <div class="room-row">
          <a href="#/room/{room.id}">{room.name}</a>
          <div class="fill" />
          {#if room.game_started}<span>Game started</span>{/if}
          {#if room.game_finished}<span>Game finished</span>{/if}
          <span>{room.number_of_player} players</span>
        </div>
      {/each}
    </div>
  {/if}
  <div style="font-weight: bold; margin-bottom: 1em;">Release notes:</div>
  <div>22.12.20: added chat and color select</div>
  <div>13.01.21: added "Ready" button</div>
</div>
