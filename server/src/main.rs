use std::collections::HashMap;
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
use log::info;

mod handler;
mod ws;
mod game;
mod model;

const HOST: &str = "127.0.0.1";
const PORT: usize = 8000;
const USER_TOKEN_HEADER: &str = "X-User-Token";
const ROOM_TTL_SEC: u64 = 600;


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
    let rooms: RoomList = Arc::new(RwLock::new(HashMap::new()));
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
                rs.retain(|_, room| {
                    let last_updated: Duration = std::time::Instant::now() - room.last_updated;
                    room.players.len() != 0 || last_updated < Duration::from_secs(ROOM_TTL_SEC)
                })
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
    let room_messages = "message";
    let room_updates = "update";
    let room_handle_routes = room
        .and(warp::post())
        .and(warp::header::<String>(USER_TOKEN_HEADER))
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

    let room_messages_routes = create_default_path(room_messages, rooms.clone(), users.clone()).and_then(handler::publish_to_room_handler);

    let room_updates_routes = create_default_path(room_updates, rooms.clone(), users.clone()).and_then(handler::update_room_state_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_user_from_token(users.clone()))
        .and(with_rooms(rooms.clone()))
        .and_then(handler::ws_handler);

    // let publish = warp::path!("publish")
    //     .and(warp::body::json())
    //     .and(with_rooms(rooms.clone()))
    //     .and_then(handler::publish_handler);
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
        .or(ws_route)
        .or(room_messages_routes)
        .or(add_user)
        .or(get_players)
        .or(game_state)
        .or(refresh_token)
        .or(validate_path)
        .or(room_updates_routes)
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