import { CONTENT_TYPE, X_USER_TOKEN } from './const';
const base_url = __environment?.isProd ? '/api' : 'http://localhost:8000';
export const createRoomRequest = async (room_name: string, usrToken) => {
    const roomCreate = await fetch(`${base_url}/room`, {
      method: `POST`,
      body: JSON.stringify({ room_name }),
      headers: {
        "X-User-Token": usrToken,
        "Content-Type": "application/json; charset=UTF-8",
      },
    });
    return roomCreate.json();
  };

  export const addUserRequest = async (name: string) => {
    const headers = new Headers();
    headers.append(CONTENT_TYPE, "application/json; charset=UTF-8");
    const user = await fetch(`${base_url}/add`, {
      method: `POST`,
      body: JSON.stringify({ name }),
      headers,
    });
    return user.json();
  }

  export const refreshTokenRequest = async (token: string) => {
    const headers = new Headers();
    headers.append(X_USER_TOKEN, token);
    return fetch(`${base_url}/refresh`, {
      method: `POST`,
      headers,
    })
      .then((user) => user.status == 200 ? user.json() : Promise.reject(user))
      .catch((err) => {
        // console.log(`error while refreshing token`, err);
        localStorage.removeItem(`x-user-token`);
        localStorage.removeItem(`x-user-id`);
        localStorage.removeItem(`x-user-name`);
        localStorage.removeItem(`x-user-id-created-at`);
        Promise.reject();
      })
  }

  export const getRoomsListRequest = async () => {
    try {
      const resp = await fetch(`${base_url}/room`, { method: `GET` });
      if (resp.status == 200) {
        return resp.json();
      } else {
        return Promise.reject([]);
      }
    } catch (err) {
      console.error(err);
      return Promise.reject([]);
    }
  }

   export const validatePathRequest = async (path: number[][], userToken: string, room_id: string) => {
    const headers = new Headers();
    headers.append(X_USER_TOKEN, userToken);
    headers.append(CONTENT_TYPE, `application/json; charset=UTF-8`);
    const validate = await fetch(
      `${base_url}/validate/${room_id}`,
      {
        method: `POST`,
        body: JSON.stringify(path),
        headers,
      }
    );
    const valid = validate.status;
    return valid === 200;
  }

  export const  makeAMoveRequest = async (path: number[][], userToken: string, room_id: string) => {
    const headers = new Headers();
    headers.append(X_USER_TOKEN, userToken);
    headers.append(CONTENT_TYPE, `application/json; charset=UTF-8`);
    await fetch(`${base_url}/message/${room_id}`, {
      method: `POST`,
      body: JSON.stringify({
        path,
        calculate_path: false,
      }),
      headers,
    });
  }

  export const startGameRequest = async (userToken: string, room_id: string) => {
    const headers = new Headers();
    headers.append(X_USER_TOKEN, userToken);
    headers.append(CONTENT_TYPE, `application/json; charset=UTF-8`);
    await fetch(`${base_url}/update/${room_id}`, {
      method: `POST`,
      body: JSON.stringify({
        start: true,
      }),
      headers,
    });
  }

  export const roomResolveRequest = async (room_id: string) => fetch(`${base_url}/room/${room_id}`)
  .then(async (res) => {
    if (res.status == 200) {
      const r = await res.json();
      // console.log(`Received room state:`, r);
      return r;
    } else {
      return Promise.reject(res);
    }
  })
  .catch(console.error);

  export const createWebSocketForRoomRequest = (userToken: string, room_id: string) => new WebSocket(
    `ws://localhost:8000/ws/${room_id}/${userToken}`
  );

  export const getRoomPlayersRequest = async (room_id: string) => {
    const r = await fetch(
        `${base_url}/players?room_id=${room_id}`
      )
        .then((res) => {
          if (res.status == 200) {
            return res.json();
          } else {
            return Promise.reject(res);
          }
        })
        .catch(console.error);
      const received = r || [];
      // console.log(received);
      return received;
  }

  export const gameStateRequest = async (room_id: string) => await fetch(
    `${base_url}/game-state?room_id=${room_id}`
  )
    .then((res) => {
      if (res.status == 200) {
        return res.json();
      } else {
        return Promise.reject(res);
      }
    })
    .catch(console.error);