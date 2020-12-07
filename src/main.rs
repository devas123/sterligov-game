use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use warp::{ws::Message, Filter, Rejection};
use std::convert::Infallible;
use std::time::Instant;
use tokio::sync::mpsc;


mod handler;
mod ws;

const HOST: &str = "127.0.0.1";
const PORT: usize = 8000;

#[derive(Debug, Clone)]
pub struct RoomHandle {
    pub room_id: String,
    pub created_by: usize,
    pub created_time: Instant,
    pub name: String,
    pub game_started: bool,
    pub game_finished: bool,
    pub active_player_id: usize,
    pub players: Vec<Player>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub user_id: usize,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>
}


type Result<T> = std::result::Result<T, Rejection>;
type RoomList = Arc<RwLock<HashMap<String, RoomHandle>>>;


#[tokio::main]
async fn main() {
    let rooms: RoomList = Arc::new(RwLock::new(HashMap::new()));
    let health_route = warp::path!("health").and_then(handler::health_handler);
    let register = warp::path("room");
    let room_handle_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::create_room_handler);
        // .or(register
        //     .and(warp::delete())
        //     .and(warp::path::param())
        //     .and(with_rooms(rooms.clone()))
        //     .and_then(handler::unregister_handler));

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

    let routes = health_route
        .or(room_handle_routes)
        .or(ws_route)
        // .or(publish)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([127, 0, 0, 1], PORT)).await;
}

fn with_rooms(rooms: RoomList) -> impl Filter<Extract = (RoomList,), Error = Infallible> + Clone {
    warp::any().map(move || rooms.clone())
}