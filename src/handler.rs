use std::convert::Infallible;
use std::error::Error;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{Buf, Rejection, Reply};
use warp::filters::ws::Message;
use warp::hyper::StatusCode;
use warp::hyper::body::Bytes;
use warp::reply::json;

use crate::{HOST, PORT, Result, RoomHandle, RoomList, ws, UserTokens};

#[derive(Debug, Clone, Deserialize)]
pub struct JoinRoomRequest {
    pub user_id: usize,
    pub room_id: usize,
}

#[derive(Deserialize, Debug)]
pub struct CreateRoomRequest {
    user_id: usize,
    room_name: String,
}

#[derive(Deserialize, Debug)]
pub struct PublishToARoomRequest {
    room_id: String,
    //TODO: move description
    message: String,
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    url: String
}

#[derive(Serialize, Debug)]
pub struct RoomDesc {
    id: String,
    name: String,
    pub created_by: usize,
    #[serde(with = "serde_millis")]
    pub created_time: Instant,
    pub game_started: bool,
    pub game_finished: bool,
    pub active_player: usize,
    pub number_of_player: usize,
}

#[derive(Debug)]
struct UserNotFound;

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

impl warp::reject::Reject for UserNotFound {}

impl RoomDesc {
    fn from_room(rh: &RoomHandle) -> RoomDesc {
        RoomDesc {
            id: rh.room_id.clone(),
            name: rh.name.clone(),
            created_by: rh.created_by,
            created_time: rh.created_time,
            game_started: rh.game_started,
            game_finished: rh.game_finished,
            active_player: rh.active_player,
            number_of_player: rh.players.len(),
        }
    }
}

pub async fn get_rooms_handler(rooms: RoomList) -> Result<impl Reply> {
    let r: Vec<RoomDesc> = rooms.read().unwrap().iter().map(|(_, v)| { RoomDesc::from_room(v) }).collect();
    Ok(json(&r))
}

pub async fn publish_to_room_handler(room_id: String, body: Bytes, rooms: RoomList, user_id_opt: Option<usize>) -> Result<impl Reply> {
    match user_id_opt {
        Some(user_id) => {
            let message = String::from_utf8(body.bytes().to_vec()).unwrap();
            publish_to_room(room_id.clone(), user_id.clone(), rooms, message).await;
            Ok(StatusCode::OK)
        }
        None => Err(warp::reject::custom(UserNotFound))
    }
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        // This error happens if the body could not be deserialized correctly
        // We can use the cause to analyze the error and customize the error message
        message = match e.source() {
            Some(cause) => {
                if cause.to_string().contains("denom") {
                    "FIELD_ERROR: denom"
                } else {
                    "BAD_REQUEST"
                }
            }
            None => "BAD_REQUEST",
        };
        code = StatusCode::BAD_REQUEST;
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else if let Some(_) = err.find::<UserNotFound>() {
        code = StatusCode::UNAUTHORIZED;
        message = "UNAUTHORIZED";
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}

pub async fn create_room_handler(body: CreateRoomRequest, rooms: RoomList) -> Result<impl Reply> {
    let user_id = body.user_id;
    let room_name = body.room_name;
    let room_id = Uuid::new_v4().simple().to_string();
    create_room(room_id.clone(), user_id.clone(), room_name, rooms).await;
    Ok(json(&CreateRoomResponse { url: format!("ws://{}:{}/ws/{}/{}", HOST, PORT, room_id, user_id) }))
}

pub async fn refresh_token_handle(userid: Option<usize>, tokens: UserTokens) -> Result<impl Reply> {
    match userid {
        None => {
            Err(warp::reject::reject())
        }
        Some(user_id) => {
            let token = Uuid::new_v4().hyphenated().to_string();
            tokens.write().unwrap().insert(token.clone(), user_id);
            Ok(warp::reply::json(&token))
        }
    }
}


async fn create_room(room_id: String, user_id: usize, room_name: String, rooms: RoomList) {
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