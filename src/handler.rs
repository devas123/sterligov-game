use warp::hyper::{StatusCode};
use crate::{RoomList, Result, RoomHandle, ws, PORT, HOST};
use warp::Reply;
use uuid::Uuid;
use std::time::Instant;
use warp::reply::json;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Deserialize)]
pub struct JoinRoomRequest {
    pub user_id: usize,
    pub room_id: usize
}

#[derive(Deserialize, Debug)]
pub struct CreateRoomRequest {
    user_id: usize,
    room_name: String
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    url: String
}

pub async fn create_room_handler(body: CreateRoomRequest, rooms: RoomList) -> Result<impl Reply> {
    let user_id = body.user_id;
    let room_name = body.room_name;
    let room_id = Uuid::new_v4().simple().to_string();
    create_room(room_id.clone(), user_id.clone(), room_name, rooms).await;
    Ok(json(&CreateRoomResponse { url: format!("ws://{}:{}/ws/{}/{}", HOST, PORT, room_id, user_id) }))
}

async fn create_room(room_id: String, user_id: usize , room_name: String, rooms: RoomList) {
    rooms.write().unwrap()
        .insert(room_id.clone(), RoomHandle {
            room_id,
            players: Vec::new(),
            name: room_name,
            active_player_id: 0,
            created_by: user_id,
            game_started: false,
            game_finished: false,
            created_time: Instant::now(),
        });
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, room_id: String, user_id: String, rooms: RoomList) -> Result<impl Reply> {
    let room = rooms.read().unwrap().get(&room_id).cloned();
    match room {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, room_id, user_id.parse().unwrap(), rooms, c))),
        None => Err(warp::reject::not_found()),
    }
}