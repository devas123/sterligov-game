use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use lru_time_cache::LruCache;
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::{Filter, Rejection, ws::Message};

mod handler;
mod ws;

const HOST: &str = "127.0.0.1";
const PORT: usize = 8000;
const USER_TOKEN_HEADER: &str = "X-User-Token";
const NEUTRAL: usize = 0;
const PURPLE: usize = 1;
const GREEN: usize = 2;
const ORANGE: usize = 3;
const YELLOW: usize = 4;
const RED: usize = 5;
const BLUE: usize = 6;
const POINT_COUNTS: [usize; 21] = [1, 2, 3, 4, 5, 16, 15, 14, 13, 12, 11, 12, 13, 14, 15, 16, 5, 4, 3, 2, 1];

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


//180 places in total.
#[derive(Debug, Clone)]
pub struct GameState {
    pub board: Vec<Vec<usize>>,
    pub cones: HashMap<(usize, usize), usize>, //(row, position, user_id)
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
}

#[derive(Debug, Clone)]
pub struct Player {
    pub user_id: usize,
    pub color: Option<usize>,
    pub name: Option<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}


type Result<T> = std::result::Result<T, Rejection>;
type RoomList = Arc<RwLock<HashMap<String, RoomHandle>>>;
type UserTokens = Arc<RwLock<LruCache<String, usize>>>;

impl GameState {
    pub fn validate_dimensions(&self, row: i32, position: i32) -> std::result::Result<(usize, usize), usize> {
        if row < 0 || row > 20 {
            return Err(0);
        }
        if position < 0 || position >= POINT_COUNTS[row as usize] as i32 {
            return Err(0);
        }
        Ok((row as usize, position as usize))
    }

    pub fn validate_cone_position(&self, row: i32, position: i32, color: usize) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, position)?;
        if let Some(c) = self.cones.get(&(r, c)) {
            if *c != color {
                return Err(0);
            }
        } else {
            return Err(0);
        }
        Ok(true)
    }

    pub fn is_occupied(&self, row: i32, position: i32) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, position)?;
        return Ok(self.cones.get(&(r, c)).is_some())
    }

    pub fn add_cone(&mut self, row: i32, col: i32, color: usize) -> std::result::Result<bool, usize> {
        let (r, c) = self.validate_dimensions(row, col)?;
        if self.cones.contains_key(&(r, c)) {
            return Err(0);
        }
        self.cones.insert((r, c).clone(), color.clone());
        Ok(true)
    }

    fn can_jump(&self, from: (i32, i32), to: (i32, i32)) -> std::result::Result<bool, usize> {
        let from_neighbors = self.get_neighbors(from.0, from.1)?;
        let to_valid = self.validate_dimensions(to.0, to.1)?;
        if from_neighbors.contains(&to_valid) || self.is_occupied(to.0, to.1)? {
            return Err(1)
        }
        let mut common_neighbors = Vec::new();
        for x in from_neighbors {
            let nn = self.get_neighbors(x.0 as i32, x.1 as i32)?;
            if nn.contains(&(from.0 as usize, from.1 as usize)) && nn.contains(&(to.0 as usize, to.1 as usize)) {
                common_neighbors.push(x);
            }
        }
        if common_neighbors.len() == 1 {
            let neighbor = common_neighbors.get(0).unwrap().clone();
                if self.is_occupied(neighbor.0 as i32, neighbor.1 as i32)? {
                    return Ok(true)
                }

        }
        Err(1)
    }

    pub fn validate_path(&self, path: Vec<(i32, i32)>) -> std::result::Result<bool, usize> {
        if path.len() < 2 {
            return Err(0);
        }
        if path.len() == 2 {
            let prev = *path.get(0).unwrap();
            let next = *path.get(1).unwrap();
            if self.is_occupied(next.0, next.1)? {
                return Err(0);
            }
            let neighbors = self.get_neighbors(prev.0, prev.1 )?;
            return if neighbors.contains(&(next.0 as usize, next.1 as usize)) {
                Ok(true)
            } else if self.can_jump(prev, next)? {
                Ok(true)
            } else {
                Err(0)
            }
        }
        for (ind, (row, col)) in path.iter().enumerate() {
            if ind == 0 {
                continue;
            }
            if self.is_occupied(*row, *col)? {
                return Err(1);
            }
            let prev = path[ind - 1];
            if !self.can_jump(prev, (*row, *col))? {
                return Err(1);
            }
        }
        Ok(true)
    }

    pub fn get_neighbors(&self, row: i32, col: i32) -> std::result::Result<HashSet<(usize, usize)>, usize> {
        let (valid_row, _) = self.validate_dimensions(row, col)?;
        let mut up_shift: i32 = if row < (POINT_COUNTS.len() - 1) as i32 { POINT_COUNTS[valid_row + 1] as i32 - POINT_COUNTS[valid_row] as i32 } else { 1 };
        let mut down_shift: i32 = if row > 0  { POINT_COUNTS[valid_row - 1] as i32 - POINT_COUNTS[valid_row] as i32 } else { 1 };
        if up_shift.abs() > 1 { up_shift = up_shift.signum() * (up_shift.abs() - 1) / 2}
        if down_shift.abs() > 1 { down_shift = down_shift.signum() * (down_shift.abs() - 1) / 2}
        let all_pos: [(i32, i32); 6] = [(row, col - 1), (row, col + 1), (row - 1, col), (row - 1, col + down_shift), (row + 1, col), (row + 1, col + up_shift)];
        // println!("down_shift: {}, up_shift: {}, all_pos: {:?}", down_shift, up_shift, all_pos);
        let mut result = HashSet::new();
        for (r, c) in all_pos.iter() {
            if self.validate_dimensions(*r, *c).is_ok() {
                if *c != col || ((*r < valid_row as i32 && down_shift.abs() <= 1) || (*r > valid_row as i32 && up_shift.abs() <= 1)) {
                    result.insert((*r as usize, *c as usize));
                }
            }
        }
        Ok(result)
    }

    pub fn get_possible_steps(&self, pos: (i32, i32)) -> std::result::Result<Vec<(usize, usize)>, &'static str> {
        match self.validate_dimensions(pos.0, pos.1) {
            Ok(_) => {
                Ok(Vec::new())
            }
            Err(_) => {
                Err("wrong dimensions.")
            }
        }
    }

    pub fn new() -> GameState {
        let points = vec![
            vec![PURPLE],
            vec![PURPLE, PURPLE],
            vec![PURPLE, PURPLE, PURPLE],
            vec![PURPLE, PURPLE, PURPLE, PURPLE],
            vec![PURPLE, PURPLE, PURPLE, PURPLE, PURPLE],
            vec![BLUE, BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN, GREEN],
            vec![BLUE, BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN, GREEN],
            vec![BLUE, BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN, GREEN],
            vec![BLUE, BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN, GREEN],
            vec![BLUE, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, GREEN],
            vec![NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL],
            vec![RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE],
            vec![RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE],
            vec![RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE],
            vec![RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE],
            vec![RED, RED, RED, RED, RED, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL, ORANGE, ORANGE, ORANGE, ORANGE, ORANGE],
            vec![YELLOW, YELLOW, YELLOW, YELLOW, YELLOW],
            vec![YELLOW, YELLOW, YELLOW, YELLOW],
            vec![YELLOW, YELLOW, YELLOW],
            vec![YELLOW, YELLOW],
            vec![YELLOW]
        ];

        for (ind, x) in points.iter().enumerate() {
            assert_eq!(x.len(), POINT_COUNTS[ind]);
        }
        GameState {
            board: points,
            cones: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() {
    let rooms: RoomList = Arc::new(RwLock::new(HashMap::new()));
    let users_count = Arc::new(AtomicUsize::new(0));
    let time_to_live = ::std::time::Duration::from_secs(3600 * 24);
    let users: UserTokens = Arc::new(RwLock::new(LruCache::<String, usize>::with_expiry_duration(time_to_live)));
    let health_route = warp::path!("health").and_then(handler::health_handler);
    let cloned_users = users.clone();
    let add_user = warp::path("add")
        .and(warp::post())
        .map(move || {
            let token = Uuid::new_v4().hyphenated().to_string();
            let new_id = users_count.clone().fetch_add(1, Ordering::Relaxed);
            cloned_users.write().unwrap().insert(token.clone(), new_id.clone());
            warp::reply::json(&token)
        });
    let refresh_token = warp::path("refresh")
        .and(warp::post())
        .and(with_userid(users.clone()))
        .and(with_users(users.clone()))
        .and_then(handler::refresh_token_handle);

    let register = warp::path("room");
    let publish = warp::path("publish");
    let room_handle_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with_rooms(rooms.clone()))
        .and_then(handler::create_room_handler)
        .or(register
            .and(warp::get())
            .and(with_rooms(rooms.clone()))
            .and_then(handler::get_rooms_handler));

    let publish_routes = publish
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

    let routes = health_route
        .or(room_handle_routes)
        .or(ws_route)
        .or(publish_routes)
        .or(add_user)
        .or(refresh_token)
        // .or(publish)
        .with(warp::cors().allow_any_origin())
        .recover(handler::handle_rejection);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn test_validate_pos() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(5, 6, YELLOW).is_ok());
        assert!(game_state.add_cone(4, 6, YELLOW).is_err());
        assert!(game_state.validate_cone_position(5, 6, YELLOW).is_ok());
        assert_ne!(true, game_state.validate_cone_position(5, 8, YELLOW).is_ok());
        assert_ne!(true, game_state.validate_cone_position(4, 8, YELLOW).is_ok());
    }

    #[test]
    fn test_can_jump() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(4, 0, YELLOW).is_ok());
        assert!(game_state.can_jump((3, 0), (5, 5)).is_ok());
        assert!(game_state.can_jump((3, 0), (3, 1)).is_err());
        assert!(game_state.can_jump((3, 0), (3, 2)).is_err());

        assert!(game_state.add_cone(3, 1, YELLOW).is_ok());
        assert!(game_state.can_jump((3, 0), (3, 2)).is_ok());
        assert!(game_state.can_jump((3, 0), (5, 6)).is_err());
    }
    #[test]
    fn test_validate_path() {
        let mut game_state = GameState::new();
        assert!(game_state.add_cone(4, 0, YELLOW).is_ok());
        assert!(game_state.validate_path(vec![(3, 0), (5, 5)]).is_ok());
        assert!(game_state.validate_path(vec![(3, 0), (5, 5), (5, 6)]).is_err());
        assert!(game_state.validate_path(vec![(3, 0), (3, 1)]).is_ok());
        assert!(game_state.validate_path(vec![(3, 1), (3, 0), (5, 5)]).is_err());
        assert!(game_state.validate_path(vec![(3, 1)]).is_err());
        assert!(game_state.validate_path(vec![(3, 1), (3, 1)]).is_err());
    }

    #[test]
    fn test_get_neighbors() {
        let game_state = GameState::new();
        println!("{:?}", game_state.get_neighbors(3, 0));
        assert_eq!(Ok(HashSet::from_iter(vec![(1, 0), (1, 1)].into_iter())), game_state.get_neighbors(0, 0));
        assert_eq!(Ok(HashSet::from_iter(vec![(12, 6), (12, 8), (11, 7), (11, 6), (13, 7), (13, 8)].into_iter())), game_state.get_neighbors(12, 7));
        assert_eq!(Ok(HashSet::from_iter(vec![(5, 1), (6, 0)].into_iter())), game_state.get_neighbors(5, 0));
        assert_eq!(Ok(HashSet::from_iter(vec![(5, 4), (5, 6), (4, 0), (6, 5), (6, 4)].into_iter())), game_state.get_neighbors(5, 5));
    }

    // #[test]
    // #[should_panic]
    // fn test_any_panic() {
    //     divide_non_zero_result(1, 0);
    // }
    //
    // #[test]
    // #[should_panic(expected = "Divide result is zero")]
    // fn test_specific_panic() {
    //     divide_non_zero_result(1, 10);
    // }
}