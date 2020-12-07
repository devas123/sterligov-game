use warp::hyper::{StatusCode};
use crate::{RoomList, Result, RoomHandle, ws, PORT, HOST};
use warp::{Reply, Buf};
use uuid::Uuid;
use std::time::Instant;
use warp::reply::json;
use serde::{Deserialize, Serialize};
use warp::filters::ws::Message;
use warp::hyper::body::Bytes;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;


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

#[derive(Deserialize, Debug)]
pub struct PublishToARoomRequest {
    room_id: String,
    //TODO: move description
    message: String
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    url: String
}

pub async fn publish_to_room_handler(room_id: String, body: Bytes, rooms: RoomList, user_id: usize) -> Result<impl Reply> {
    let message = String::from_utf8(body.bytes().to_vec()).unwrap();
    publish_to_room(room_id.clone(), user_id.clone(), rooms, message).await;
    Ok(StatusCode::OK)
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
            active_player: 0,
            created_by: user_id,
            game_started: false,
            game_finished: false,
            created_time: Instant::now(),
        });
}

async fn publish_to_room(room_id: String, user_id: usize, rooms: RoomList, message: String) {

    let mut message_sent = false;
    println!("publish to room: {}, user_id: {}, message: {}", room_id, user_id, message);
    if let Some(r) = rooms.clone().read().unwrap().get(&room_id) {
        println!("Found the room: {}, created_by {} at {:?}", r.name, r.created_by, r.created_time);
        for (ind, player) in r.players.iter().enumerate() {
            println!("Looking at player {}, current turn is: {}", ind, r.active_player);
            if user_id == player.user_id && ind == r.active_player {
                println!("Player {} can make a move.", ind);
                for x in &r.players {
                    if let Some(sender) = &x.sender {
                        if let Err(x) = sender.send(Ok(Message::text(message.clone()))) {
                            println!("Error while sending to player: {}", x);
                        };
                        message_sent = true;
                    }
                }
                break;
            }
        }
        if message_sent {
            println!("Message sent, taking turns.");
            match rooms.try_write() {
                Ok(mut result) => {
                    result.insert(room_id.clone(), r.incr_active_player());
                }
                Err(err) => {
                    println!("Error while acquiring the lock: {}", err)
                }
            }
        }
    }
    println!("Finished.");
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