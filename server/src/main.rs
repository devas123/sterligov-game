use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::env;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize};

use lru_time_cache::LruCache;
use warp::{Filter, Rejection};

use model::RoomDesc;
use serde::de::DeserializeOwned;
use model::{RoomHandle, User};
use tokio::time::{Duration, Instant};
use std::ops::Add;
use log::{info};
use crate::model::{Player, Message};
use crate::ws::{send_update, PlayerLeftUpdate};
use std::cmp::max;

mod handler;
mod ws;
mod game;
mod model;

const HOST: &str = "127.0.0.1";
const PORT: usize = 8000;
const USER_TOKEN_HEADER: &str = "X-User-Token";
const ROOM_TTL_SEC: u64 = 60;
const PLAYER_TTL_SEC: u64 = 40;


type Result<T> = std::result::Result<T, Rejection>;
type RoomList = Arc<RwLock<HashMap<String, RoomHandle>>>;
type UserTokens = Arc<RwLock<LruCache<String, User>>>;

fn create_default_path<T>(path: &'static str, rooms: RoomList, users: UserTokens) -> impl Filter<Extract=(String, T, RoomList, Option<usize>, ), Error=Rejection> + Clone
where T: DeserializeOwned + Send {
    return warp::path(path)
        .and(warp::post())
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_rooms(rooms))
        .and(with_userid(users))
}

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();
    let rooms = Arc::new(RwLock::new(HashMap::new()));
    let users_count = Arc::new(AtomicUsize::new(0));
    let time_to_live = ::std::time::Duration::from_secs(3600 * 24);
    let users: UserTokens = Arc::new(RwLock::new(LruCache::<String, User>::with_expiry_duration(time_to_live)));
    let health_route = warp::path!("health").and_then(handler::health_handler);
    let mut interval = tokio::time::interval_at(Instant::now().add(Duration::from_secs(ROOM_TTL_SEC)), Duration::from_secs(ROOM_TTL_SEC));
    let rooms_cloned = rooms.clone();
    tokio::spawn( async move {
        loop {
            interval.tick().await;
            if let Ok(mut rs) = rooms_cloned.try_write() {
                info!("Removing stale rooms.");
                rs.retain(|_, room: &mut RoomHandle| {
                    let last_updated: Duration = std::time::Instant::now() - room.last_updated;
                    room.players.len() != 0 || last_updated < Duration::from_secs(ROOM_TTL_SEC)
                });

                for (_, handler) in rs.iter_mut() {
                    for p in handler.players.iter_mut() {
                        if p.sender.send(Ok(Message::event("test".to_string()))).is_ok() {
                            p.last_active = std::time::Instant::now();
                        }
                    }
                    handler.players.retain(|p: &Player| {
                        std::time::Instant::now() - p.last_active < Duration::from_secs(PLAYER_TTL_SEC)
                    });
                    let players = &handler.players;
                    let mut removed_players = Vec::new();
                    let mut removed_colors = HashSet::new();
                    let room_id = handler.room_id.clone();
                    let game_started = handler.game_started;
                    handler.active_player %= max(handler.players.len(), 1);
                    if let Some(gs) = handler.game_state.as_mut() {
                        gs.players_colors.retain(|id, c| {
                            let valid = players.iter().any(|p|{ p.user_id == *id});
                            if !valid {
                                removed_players.push((*id, *c));
                                removed_colors.insert(*c);
                            }
                            valid
                        });
                        if !game_started {
                           gs.cones.retain(|(_, _), color| { !removed_colors.contains(color) })
                        }
                    }
                    for (user_id, player_color) in removed_players.iter() {
                        send_update(handler, &PlayerLeftUpdate::new(*user_id, room_id.clone(), handler.active_player, !game_started, *player_color));
                    }
                }
            } else {
                info!("Could not acquire lock for removing stale rooms.");
            }
        }
    });
    let validate_path = warp::path("validate")
        .and(warp::post())
        .and(warp::path::param())
        .and(with_rooms(rooms.clone()))
        .and(with_userid(users.clone()))
        .and(warp::body::json())
        .and_then(handler::validate_path);
    let get_players = warp::path("players")
        .and(warp::get())
        .and(warp::query())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::get_players);
    let game_state = warp::path("game-state")
        .and(warp::get())
        .and(warp::query())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::get_game_state);
    let add_user = warp::path("add")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::json())
        .and(with_users(users.clone()))
        .and(with_users_counter(users_count.clone()))
        .and_then(handler::add_user_handle);
    let refresh_token = warp::path("refresh")
        .and(warp::post())
        .and(with_user(users.clone()))
        .and(with_users(users.clone()))
        .and_then(handler::refresh_token_handle);

    let room = warp::path("room");
    let room_moves = "move";
    let room_messages = "chat";
    let room_updates = "update";
    let room_handle_routes = room
        .and(warp::post())
        .and(with_userid(users.clone()))
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::create_room_handler)
        .or(room
            .and(warp::get())
            .and(warp::path::param())
            .and(with_rooms(rooms.clone()))
            .map(|room_id: String, rooms: RoomList| {
                warp::reply::json(&rooms.read().unwrap().get(&room_id).map(RoomDesc::from_room))
            }))
        .or(room
            .and(warp::get())
            .and(with_rooms(rooms.clone()))
            .and_then(handler::get_rooms_handler));

    let room_messages_routes = create_default_path(room_moves, rooms.clone(), users.clone()).and_then(handler::make_a_move_handler);

    let room_updates_routes = create_default_path(room_updates, rooms.clone(), users.clone()).and_then(handler::update_room_state_handler);
    let room_chat_routes = warp::path(room_messages)
        .and(warp::post())
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and(with_user(users.clone()))
        .and_then(handler::room_chat_message_handler);

    let sse_route = warp::path("sse")
        .and(warp::get())
        .and(warp::path::param())
        .and(with_user_from_token(users.clone()))
        .and(with_rooms(rooms.clone()))
        .and_then(|room_id: String, user: Option<User>, rooms: RoomList| async {
            handler::sse_handler(room_id, user, rooms).map(|stream| { warp::sse::reply(warp::sse::keep_alive().stream(stream)) })
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["POST", "GET", "DELETE", "OPTIONS"])
        .allow_headers(vec!["content-type",
                            USER_TOKEN_HEADER,
                            "Content-Length",
                            "Sec-Fetch-Dest",
                            "Sec-Fetch-Mode",
                            "Sec-Fetch-Site"
        ]);
    let routes = health_route
        .or(room_handle_routes)
        .or(room_messages_routes)
        .or(add_user)
        .or(get_players)
        .or(game_state)
        .or(refresh_token)
        .or(validate_path)
        .or(room_updates_routes)
        .or(room_chat_routes)
        .or(sse_route)
        // .or(publish)
        .with(cors)
        .recover(handler::handle_rejection)
        .map(|reply| {
            warp::reply::with_header(reply, "Access-Control-Allow-Origin", "*")
        })
        .with(warp::log::log("tests"));


    warp::serve(routes).run(([127, 0, 0, 1], PORT as u16)).await;
}

fn with_rooms(rooms: RoomList) -> impl Filter<Extract=(RoomList, ), Error=Infallible> + Clone {
    warp::any().map(move || rooms.clone())
}

fn with_users(users: UserTokens) -> impl Filter<Extract=(UserTokens, ), Error=Infallible> + Clone {
    warp::any().map(move || users.clone())
}

fn with_users_counter(userscounts: Arc<AtomicUsize>) -> impl Filter<Extract=(Arc<AtomicUsize>, ), Error=Infallible> + Clone {
    warp::any().map(move || userscounts.clone())
}

fn with_userid(users: UserTokens) -> impl Filter<Extract=(Option<usize>, ), Error=Rejection> + Clone {
    warp::header::optional(USER_TOKEN_HEADER).map(move |token: Option<String>| {
        token.map(|t| {
            users.write().unwrap().get(&t).map(|user| { user.user_id })
        }).flatten()
    })
}

fn with_user_from_token(users: UserTokens) -> impl Filter<Extract=(Option<User>, ), Error=Rejection> + Clone {
    warp::path::param().map(move |token: String| {
        users.write().unwrap().get(&token).map(|user| { user }).cloned()
    })
}

fn with_user(users: UserTokens) -> impl Filter<Extract=(Option<User>, ), Error=Rejection> + Clone {
    warp::header::optional(USER_TOKEN_HEADER).map(move |token: Option<String>| {
        token.map(|t| {
            users.write().unwrap().get(&t).cloned()
        }).flatten()
    })
}