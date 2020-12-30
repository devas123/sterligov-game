use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use futures::Stream;
use log::{error, info};
use uuid::Uuid;
use warp::{Rejection, Reply};
use warp::filters::cors::CorsForbidden;
use warp::filters::sse::ServerSentEvent;
use warp::hyper::StatusCode;
use warp::reply::json;

use crate::{HOST, PORT, Result, RoomHandle, RoomList, User, UserTokens, ws};
use crate::game::{GameState, NEUTRAL};
use crate::model::{AddUserRequest, CreateRoomRequest, CreateRoomResponse, ErrorMessage, GameColorsUpdate, PlayerDesc, PublishToARoomRequest, RoomDesc, RoomFull, RoomIdParameter, RoomNotFound, RoomStateUpdate, TokenCreatedResponse, UpdateRoomStateRequest, UpdateRoomType, UserNotFound};
use crate::model::UpdateRoomType::{ColorChange, Start, Stop};
use crate::ws::{ChatMessage, PlayerLeftUpdate, send_update, SendMessageRequest};
use std::cmp::max;
use std::collections::HashSet;

pub async fn get_rooms_handler(rooms: RoomList) -> Result<impl Reply> {
    let mut r: Vec<RoomDesc> = rooms.read().unwrap().iter().map(|(_, v)| { RoomDesc::from_room(v) }).collect();
    r.sort_by(|k, p| { p.created_time.cmp(&k.created_time) });
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

pub async fn room_chat_message_handler(room_id: String, body: SendMessageRequest, rooms: RoomList, user: Option<User>) -> Result<impl Reply> {
    if let Some(usr) = user {
        if let Ok(lock) = rooms.try_read() {
            if let Some(room) = lock.get(&room_id) {
                send_update(room, &ChatMessage::new(usr.user_name.as_str(), usr.user_id, body.message.as_str()));
                Ok(StatusCode::OK)
            } else {
                Err(warp::reject::custom(RoomNotFound))
            }
        } else {
            error!("Log cannot be acquired");
            Err(warp::reject::custom(RoomNotFound))
        }
    } else {
        Err(warp::reject::custom(UserNotFound))
    }
}

pub async fn make_a_move_handler(room_id: String, body: PublishToARoomRequest, rooms: RoomList, user_id_opt: Option<usize>) -> Result<impl Reply> {
    match user_id_opt {
        Some(user_id) => {
            make_a_move(room_id.clone(), user_id.clone(), rooms, body).await?;
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
        message = "User was not found";
    } else if let Some(_) = err.find::<RoomNotFound>() {
        code = StatusCode::BAD_REQUEST;
        message = "Room was not found";
    } else if let Some(_) = err.find::<RoomFull>() {
        code = StatusCode::BAD_REQUEST;
        message = "Room room is full";
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

pub async fn create_room_handler(user_id_opt: Option<usize>, body: CreateRoomRequest, rooms: RoomList) -> Result<impl Reply> {
    let user_id = match user_id_opt {
        None => {
            return Err(warp::reject::reject());
        }
        Some(id) => { id }
    };
    let room_name = body.room_name;
    if room_name.is_empty() || room_name.len() > 15 {
        Err(warp::reject::reject())
    } else {
        let room_id = Uuid::new_v4().simple().to_string();
        let room = create_room(room_id.clone(), user_id.clone(), room_name, rooms).await;
        Ok(json(&CreateRoomResponse {
            room,
            url: format!("http://{}:{}/sse/{}", HOST, PORT, room_id.clone()),
        }))
    }
}

pub async fn get_players(query: RoomIdParameter, rooms: RoomList) -> Result<impl Reply> {
    let room_id = query.room_id;
    rooms.read().unwrap().get(&room_id)
        .map(|room| {
            room.players.iter().map(|player| {
                PlayerDesc::from_player(player, room.game_state.as_ref().map(|gs| { gs.players_colors.get(&player.user_id).cloned() }).flatten().unwrap())
            }).collect()
        })
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
            Ok(warp::reply::json(&TokenCreatedResponse { token, created_at: Instant::now(), user_id, user_name }))
        }
    }
}

pub async fn add_user_handle(request: AddUserRequest, users: UserTokens, users_counts: Arc<AtomicUsize>) -> Result<impl Reply> {
    if request.name.is_empty() || request.name.len() > 15 {
        Err(warp::reject())
    } else {
        let token = Uuid::new_v4().hyphenated().to_string();
        let new_id = users_counts.as_ref().fetch_add(1, Ordering::Relaxed);
        users.write().unwrap().insert(token.clone(), User { user_id: new_id.clone(), user_name: request.name.clone() });
        Ok(warp::reply::json(&TokenCreatedResponse { token, created_at: Instant::now(), user_id: new_id, user_name: request.name }))
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
        last_updated: Instant::now(),
        game_state: Some(GameState::new()),
    };
    let desc = RoomDesc::from_room(&handle);
    rooms.write().unwrap()
        .insert(room_id, handle);
    desc
}

async fn make_a_move(room_id: String, user_id: usize, rooms: RoomList, request: PublishToARoomRequest) -> Result<&'static str> {
    info!("Make a move, room: {}, user_id: {}, message: {:?}", room_id, user_id, request);
    if request.path.len() < 2 {
        error!("Path too short.");
        return Err(warp::reject());
    } else {
        for x in request.path.iter() {
            if x.len() != 2 {
                error!("Path invalid.");
                return Err(warp::reject());
            }
        }
    }
    let transformed: Vec<(i32, i32)> = request.path.iter().map(|v| { (v[0], v[1]) }).collect();
    if let Some(r) = rooms.clone().try_write().unwrap().get_mut(&room_id) {
        info!("Found the room: {}, created_by {} at {:?}", r.name, r.created_by, r.created_time);
        for (ind, player) in r.players.iter().enumerate() {
            info!("Looking at player (user_id: {}, number {}), current turn is: {}", player.user_id, ind, r.active_player);
            if user_id == player.user_id && ind == r.active_player {
                info!("Player {} can make a move.", ind);
                match r.make_a_move(transformed, user_id) {
                    Ok(msg) => {
                        send_update(r, &msg);
                        r.last_updated = Instant::now();
                        break;
                    }
                    Err(e) => {
                        error!("Error while making a move: {}", e);
                        return Err(warp::reject());
                    }
                }
            }
        }
    } else {
        error!("Message not sent!!!!!111");
        return Err(warp::reject());
    }
    info!("Finished.");
    Ok("ok")
}

async fn update_room_state(room_id: String, user_id: usize, rooms: RoomList, request: UpdateRoomStateRequest) {
    info!("Update room state: {}, user_id: {}, message: {:?}", room_id, user_id, request);
    if let Some(r) = rooms.clone().write().unwrap().get_mut(&room_id) {
        info!("Found the room: {}, created_by {} at {:?}", r.name, r.created_by, r.created_time);
        match request.update_type {
            Start | Stop => {
                if r.created_by == user_id {
                    r.game_started = request.update_type == Start;
                    send_update(r, &RoomStateUpdate::new(r));
                } else {
                    error!("User {} did not create the room, he can't update the state.", user_id)
                }
            }
            ColorChange => {
                if !r.game_started {
                    request.new_color.map(move |new_color| {
                        if new_color > 0 && new_color < 7 {
                            if let Some(gs) = r.game_state.as_mut() {
                                if !gs.players_colors.iter().any(|(id, color)| { *color == new_color && *id != user_id }) {
                                    let mut old = 0;
                                    if let Some(old_color) = gs.players_colors.insert(user_id, new_color) {
                                        gs.cones.retain(|(_, _), color| { *color != old_color });
                                        old = old_color;
                                    }
                                    if let Ok(_) = gs.add_cones(new_color) {
                                        let new_gs = GameState {
                                            cones: gs.cones.clone(),
                                            players_colors: gs.players_colors.clone(),
                                            moves: gs.moves.clone(),
                                        };
                                        let update = GameColorsUpdate::new(r.room_id.as_str(), new_gs);
                                        send_update(r, &update);
                                    } else if old > 0 {
                                        error!("Error when adding cones to the board.");
                                        if let Err(_) = gs.add_cones(old) {
                                            error!("Error when adding old cones too :(");
                                        }
                                        gs.players_colors.insert(user_id, old);
                                    }
                                }
                            }
                        }
                    });
                }
            }
            UpdateRoomType::Leave => {
                let mut player_color = NEUTRAL;
                r.players.retain(|p| { p.user_id != user_id });
                if !r.game_started {
                    if let Some(gs) = r.game_state.as_mut() {
                        let mut removed_colors = HashSet::new();
                        player_color = *gs.players_colors.get(&user_id).unwrap_or_else(|| &NEUTRAL);
                        gs.players_colors.retain(|pl, c| {
                           if *pl != user_id {
                               removed_colors.insert(*c);
                               true
                           } else {
                               false
                           }
                        });
                        gs.cones.retain(|(_,_), c| { !removed_colors.contains(c) });
                    }
                }
                r.active_player %= max(r.players.len(), 1);
                send_update(r, &PlayerLeftUpdate::new(user_id, room_id.clone(), r.active_player, !r.game_started, player_color));
            }
        }
    }
    info!("Finished.");
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}

pub fn sse_handler(room_id: String, user: Option<User>, rooms: RoomList) -> Result<impl Stream<Item=std::result::Result<impl ServerSentEvent + Send + 'static, warp::Error>> + Send + 'static> {
    if let Some(r) = rooms.clone().write().unwrap().get_mut(&room_id) {
        if let Some(usr) = user {
            ws::client_connection(room_id, usr, r)
        } else {
            Err(warp::reject::custom(UserNotFound))
        }
    } else {
        Err(warp::reject::custom(RoomNotFound))
    }
}