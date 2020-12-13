use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use lru_time_cache::LruCache;
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::{Filter, Rejection, ws::Message};

use crate::game::GameState;

mod handler;
mod ws;
mod game;

const HOST: &str = "127.0.0.1";
const PORT: usize = 8000;
const USER_TOKEN_HEADER: &str = "X-User-Token";

#[derive(Debug, Clone)]
pub struct RoomHandle {
    pub room_id: String,
    pub created_by: usize,
    pub created_time: Instant,
    pub name: String,
    pub game_started: bool,
    pub game_finished: bool,
    pub active_player: usize,
    pub game_state: Option<GameState>,
    pub players: Vec<Player>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoomUpdate {
    name: String,
    pub by_user_id: usize,
    pub path: Vec<(usize, usize)>,
    pub next_player: usize,
}

impl RoomUpdate {
    fn new(by_user_id: usize,
           path: Vec<(usize, usize)>,
           next_player: usize,) -> RoomUpdate {
        RoomUpdate {
            name: "room_update".to_string(),
            by_user_id,
            path,
            next_player
        }
    }
}


impl RoomHandle {
    pub fn incr_active_player(&self) -> RoomHandle {
        let mut rh = self.clone();
        rh.active_player = (rh.active_player + 1) % rh.players.len();
        rh
    }

    pub fn remove_player(&self, user_id: usize) -> Option<RoomHandle> {
        let mut rh = self.clone();
        let mut players = Vec::new();
        for p in rh.players {
            if p.user_id != user_id {
                players.push(p);
            }
        }
        rh.players = players;
        if rh.players.len() > 0 {
            Some(rh)
        } else {
            None
        }
    }

    pub fn make_a_move(&self, path: Vec<(i32, i32)>, user_id: usize) -> std::result::Result<(RoomUpdate, RoomHandle), usize> {
        let mut rh = self.clone();
        if let Some(gs) = rh.game_state.as_mut() {
            let next = (rh.active_player + 1) % rh.players.len();
            let p = (path[0].0 as usize, path[0].1 as usize);
            if *gs.cones.get(&p).unwrap() == user_id {
                let update = gs.update_cones(path.clone())
                    .map(move |path: Vec<(usize, usize)>| { RoomUpdate::new( user_id, path, next.clone()) });
                rh.active_player = next;
                return update.map(|u| { (u, rh) });
            }
        }
        Err(0)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub user_id: usize,
    pub color: Option<usize>,
    pub name: Option<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[derive(Serialize)]
pub struct TokenCreatedResponse {
    pub token: String,
    #[serde(with = "serde_millis")]
    pub created_at: Instant,
}


type Result<T> = std::result::Result<T, Rejection>;
type RoomList = Arc<RwLock<HashMap<String, RoomHandle>>>;
type UserTokens = Arc<RwLock<LruCache<String, usize>>>;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let rooms: RoomList = Arc::new(RwLock::new(HashMap::new()));
    let users_count = Arc::new(AtomicUsize::new(0));
    let time_to_live = ::std::time::Duration::from_secs(3600 * 24);
    let users: UserTokens = Arc::new(RwLock::new(LruCache::<String, usize>::with_expiry_duration(time_to_live)));
    let health_route = warp::path!("health").and_then(handler::health_handler);
    let cloned_users = users.clone();
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
        .map(move || {
            let token = Uuid::new_v4().hyphenated().to_string();
            let new_id = users_count.clone().fetch_add(1, Ordering::Relaxed);
            cloned_users.write().unwrap().insert(token.clone(), new_id.clone());
            warp::reply::json(&TokenCreatedResponse { token, created_at: Instant::now() })
        });
    let refresh_token = warp::path("refresh")
        .and(warp::post())
        .and(with_userid(users.clone()))
        .and(with_users(users.clone()))
        .and_then(handler::refresh_token_handle);

    let room = warp::path("room");
    let room_messages = warp::path("message");
    let room_handle_routes = room
        .and(warp::post())
        .and(with_userid(users.clone()))
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::create_room_handler)
        .or(room
            .and(warp::get())
            .and(with_rooms(rooms.clone()))
            .and_then(handler::get_rooms_handler));

    let room_messages_routes = room_messages
        .and(warp::post())
        .and(warp::path::param())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and(with_userid(users.clone()))
        .and_then(handler::publish_to_room_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(warp::path::param())
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

fn with_userid(users: UserTokens) -> impl Filter<Extract=(Option<usize>, ), Error=Rejection> + Clone {
    warp::header::optional(USER_TOKEN_HEADER).map(move |token: Option<String>| {
        token.map(|t| {
            users.write().unwrap().get(&t).cloned()
        }).flatten()
    })
}