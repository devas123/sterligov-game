use std::convert::Infallible;
use std::error::Error;
use std::time::Instant;

use log::{error, info};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{Rejection, Reply};
use warp::filters::cors::CorsForbidden;
use warp::hyper::StatusCode;
use warp::reply::json;

use crate::{HOST, Player, PORT, Result, RoomHandle, RoomList, TokenCreatedResponse, UserTokens, ws, User, RoomStateUpdate};
use crate::game::GameState;
use crate::ws::send_update;

#[derive(Debug, Clone, Deserialize)]
pub struct JoinRoomRequest {
    pub user_id: usize,
    pub room_id: usize,
}

#[derive(Deserialize, Debug)]
pub struct CreateRoomRequest {
    room_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct PublishToARoomRequest {
    path: Vec<Vec<i32>>,
    calculate_path: bool
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateRoomStateRequest {
    start: bool
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    room: RoomDesc,
    url: String,
    url_sockjs: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct RoomDesc {
    id: String,
    name: String,
    pub winner: Option<usize>,
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

#[derive(Serialize)]
struct PlayerDesc {
    name: String,
    color: usize,
    user_id: usize,
}

#[derive(Deserialize)]
pub struct RoomIdParameter {
    pub room_id: String
}

impl warp::reject::Reject for UserNotFound {}

impl PlayerDesc {
    fn from_player(p: &Player, color: usize) -> PlayerDesc {
        PlayerDesc {
            name: p.name.as_ref().cloned().or_else(|| { Some("Player".to_string()) }).unwrap(),
            color,
            user_id: p.user_id,
        }
    }
}

impl RoomDesc {
    pub fn from_room(rh: &RoomHandle) -> RoomDesc {
        RoomDesc {
            id: rh.room_id.clone(),
            winner: rh.winner.clone(),
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

pub async fn update_room_state_handler(room_id: String, body: UpdateRoomStateRequest, rooms: RoomList, user_id_opt: Option<usize>) -> Result<impl Reply> {
    match user_id_opt {
        Some(user_id) => {
            update_room_state(room_id.clone(), user_id.clone(), rooms, body).await;
            Ok(StatusCode::OK)
        }
        None => Err(warp::reject::custom(UserNotFound))
    }
}

pub async fn publish_to_room_handler(room_id: String, body: PublishToARoomRequest, rooms: RoomList, user_id_opt: Option<usize>) -> Result<impl Reply> {
    match user_id_opt {
        Some(user_id) => {
            publish_to_room(room_id.clone(), user_id.clone(), rooms, body).await;
            Ok(StatusCode::OK)
        }
        None => Err(warp::reject::custom(UserNotFound))
    }
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let message;
    error!("Got an error: {:?}", err);
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
    } else if let Some(_) = err.find::<CorsForbidden>() {
        code = StatusCode::BAD_REQUEST;
        message = "Header not allowed";
    } else {
        // We should have expected this... Just log and say its a 500
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}

pub async fn create_room_handler(user_token: String, user_id_opt: Option<usize>, body: CreateRoomRequest, rooms: RoomList) -> Result<impl Reply> {
    let user_id = match user_id_opt {
        None => {
            return Err(warp::reject::reject());
        }
        Some(id) => { id }
    };
    let room_name = body.room_name;
    let room_id = Uuid::new_v4().simple().to_string();
    let room = create_room(room_id.clone(), user_id.clone(), room_name, rooms).await;
    Ok(json(&CreateRoomResponse {
        room,
        url: format!("ws://{}:{}/ws/{}/{}", HOST, PORT, room_id.clone(), user_token),
        url_sockjs: format!("http://{}:{}/ws/{}/{}", HOST, PORT, room_id.clone(), user_token),
    }))
}

pub async fn get_players(query: RoomIdParameter, rooms: RoomList) -> Result<impl Reply> {
    let room_id = query.room_id;
    rooms.read().unwrap().get(&room_id)
        .map(|room| { room.players.iter().map(|player| {
            PlayerDesc::from_player(player, room.game_state.as_ref().map(|gs| {gs.players_colors.get(&player.user_id).cloned()}).flatten().unwrap())
        }).collect() })
        .map(|players: Vec<PlayerDesc>| { json(&players) })
        .ok_or(warp::reject::reject())
}

pub async fn validate_path(room_id: String, rooms: RoomList, _userid: Option<usize>, body: Vec<Vec<i32>>) -> Result<impl Reply> {
    let transformed: Vec<(i32, i32)> = body.iter().map(|x| { (x[0], x[1]) }).collect();
    if let Some(result) = rooms.read().unwrap().get(&room_id)
        .map(|room| {
            room.game_state.as_ref().map(|gs| {
                gs.validate_path(&transformed)
                    .map_err(|_e| {
                        error!("Path invalid: {:?}", transformed);
                        warp::reject()
                    })
            })
        })
        .flatten() {
        result.map(|x| { if x { Ok(StatusCode::OK) } else { Ok(StatusCode::NOT_ACCEPTABLE) } })
    } else {
        error!("Path invalid: {:?}", transformed);
        Err(warp::reject())
    }
}

pub async fn get_game_state(query: RoomIdParameter, rooms: RoomList) -> Result<impl Reply> {
    let room_id = query.room_id;
    rooms.read().unwrap().get(&room_id)
        .map(|room| { room.game_state.as_ref() })
        .flatten()
        .map(|ogs| { json(ogs) })
        .ok_or(warp::reject::reject())
}

pub async fn refresh_token_handle(user: Option<User>, tokens: UserTokens) -> Result<impl Reply> {
    match user {
        None => {
            Err(warp::reject::reject())
        }
        Some(usr) => {
            let token = Uuid::new_v4().hyphenated().to_string();
            let user_name = usr.user_name.clone();
            let user_id = usr.user_id.clone();
            tokens.write().unwrap().insert(token.clone(), usr);
            Ok(warp::reply::json(&TokenCreatedResponse { token, created_at: Instant::now(), user_id, user_name}))
        }
    }
}


async fn create_room(room_id: String, user_id: usize, room_name: String, rooms: RoomList) -> RoomDesc {
    let handle = RoomHandle {
        winner: None,
        room_id: room_id.clone(),
        players: Vec::new(),
        name: room_name,
        active_player: 0,
        created_by: user_id,
        game_started: false,
        game_finished: false,
        created_time: Instant::now(),
        game_state: Some(GameState::new()),
    };
    let desc = RoomDesc::from_room(&handle);
    rooms.write().unwrap()
        .insert(room_id, handle);
    desc
}

async fn publish_to_room(room_id: String, user_id: usize, rooms: RoomList, request: PublishToARoomRequest) {
    info!("Make a move, room: {}, user_id: {}, message: {:?}", room_id, user_id, request);
    let transformed: Vec<(i32, i32)> = request.path.iter().map(|v| { (v[0], v[1]) }).collect();
    if let Some(r) = rooms.clone().try_write().unwrap().get_mut(&room_id) {
        info!("Found the room: {}, created_by {} at {:?}", r.name, r.created_by, r.created_time);
        for (ind, player) in r.players.iter().enumerate() {
            info!("Looking at player {}, current turn is: {}", ind, r.active_player);
            if user_id == player.user_id && ind == r.active_player {
                info!("Player {} can make a move.", ind);
                if let Ok(msg) = r.make_a_move(transformed, user_id) {
                    send_update(r, &msg);
                } else {
                    error!("Error while making a move.")
                }
                break;
            }
        }
    } else {
        error!("Message not sent!!!!!111")
    }
    info!("Finished.");
}

async fn update_room_state(room_id: String, user_id: usize, rooms: RoomList, request: UpdateRoomStateRequest) {
    info!("Update room state: {}, user_id: {}, message: {:?}", room_id, user_id, request);
    if let Some(r) = rooms.clone().write().unwrap().get_mut(&room_id) {
        info!("Found the room: {}, created_by {} at {:?}", r.name, r.created_by, r.created_time);
        if r.created_by == user_id {
            r.game_started = request.start;
            send_update(r, &RoomStateUpdate::new(r));
        } else {
            error!("User {} did not create the room, he can't update the state.", user_id)
        }
    }
    info!("Finished.");
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, room_id: String, user: Option<User>, rooms: RoomList) -> Result<impl Reply> {
    let room = rooms.read().unwrap().get(&room_id).cloned();
    match room {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, room_id, user.unwrap(), rooms, c))),
        None => Err(warp::reject::not_found())
    }
}