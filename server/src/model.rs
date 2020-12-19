use std::time::Instant;
use crate::game::GameState;
use tokio::sync::mpsc;
use warp::filters::ws::Message;
use serde::{Serialize, Deserialize};
use log::{error};

#[derive(Debug, Clone)]
pub struct RoomHandle {
    pub room_id: String,
    pub winner: Option<usize>,
    pub created_by: usize,
    pub created_time: Instant,
    pub name: String,
    pub game_started: bool,
    pub game_finished: bool,
    pub active_player: usize,
    pub game_state: Option<GameState>,
    pub players: Vec<Player>,
}

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub name: String
}

#[derive(Debug, Clone, Serialize)]
pub struct RoomUpdate {
    name: String,
    pub by_user_id: usize,
    pub path: Vec<(usize, usize)>,
    pub next_player: usize,
    pub game_finished: bool
}

impl RoomUpdate {
    fn new_with_finished(by_user_id: usize,
           path: Vec<(usize, usize)>,
           next_player: usize,
           game_finished: bool) -> RoomUpdate {
        RoomUpdate {
            name: "move_made".to_string(),
            by_user_id,
            path,
            next_player,
            game_finished
        }
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct RoomStateUpdate {
    name: String,
    pub room: RoomDesc
}

impl RoomStateUpdate {
    pub fn new(room: &RoomHandle) -> RoomStateUpdate {
        RoomStateUpdate {
            name: "room_state_update".to_string(),
            room: RoomDesc::from_room(room)
        }
    }
}


impl RoomHandle {
    pub fn remove_player(&mut self, user_id: usize) {
        for (ind, p) in self.players.iter().enumerate() {
            if p.user_id == user_id {
                self.players.remove(ind);
                break;
            }
        }
    }

    pub fn make_a_move(&mut self, path: Vec<(i32, i32)>, user_id: usize) -> std::result::Result<RoomUpdate, usize> {
        if let Some(gs) = self.game_state.as_mut() {
            let next = (self.active_player + 1) % self.players.len();
            let p = (path[0].0 as usize, path[0].1 as usize);
            if let Some(user_color) = gs.players_colors.get(&user_id) {
                if let Some(color) = gs.cones.get(&p) {
                    if *color == *user_color {
                        let update = gs.update_cones(&path, &user_id)
                            .map(|(path, game_finished)| {
                                self.active_player = next;
                                if game_finished {
                                    self.winner = Some(user_id.clone());
                                    self.game_finished = true;
                                }
                                RoomUpdate::new_with_finished(user_id, path, next.clone(), game_finished)
                            });
                        return update;
                    }
                } else {
                    error!("Could not find user {} in cones at position: {:?}. Cones: {:?}", user_id, p, gs.cones);
                }
            }
        }
        Err(0)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub user_id: usize,
    pub name: Option<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[derive(Serialize)]
pub struct TokenCreatedResponse {
    pub token: String,
    #[serde(with = "serde_millis")]
    pub created_at: Instant,
    pub user_id: usize,
    pub user_name: String,
}

#[derive(Clone)]
pub struct User {
    pub user_id: usize,
    pub user_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JoinRoomRequest {
    pub user_id: usize,
    pub room_id: usize,
}

#[derive(Deserialize, Debug)]
pub struct CreateRoomRequest {
    pub room_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct PublishToARoomRequest {
    pub path: Vec<Vec<i32>>,
    pub calculate_path: bool
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateRoomStateRequest {
    pub start: bool
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    pub room: RoomDesc,
    pub url: String,
    pub url_sockjs: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct RoomDesc {
    pub id: String,
    pub name: String,
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
pub struct UserNotFound;

#[derive(Serialize)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
}

#[derive(Serialize)]
pub struct PlayerDesc {
    pub name: String,
    pub color: usize,
    pub user_id: usize,
}

#[derive(Deserialize)]
pub struct RoomIdParameter {
    pub room_id: String
}

impl warp::reject::Reject for UserNotFound {}

impl PlayerDesc {
    pub fn from_player(p: &Player, color: usize) -> PlayerDesc {
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
