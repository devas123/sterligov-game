use std::time::Instant;
use crate::game::GameState;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use log::{error};
use crate::model::Message::{Event};

#[derive(Debug)]
pub struct RoomHandle {
    pub room_id: String,
    pub winner: Option<usize>,
    pub created_by: usize,
    pub created_time: Instant,
    pub last_updated: Instant,
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
    pub room: RoomDesc,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveTimerUpdate {
    name: String,
    pub timer_value: usize,
    pub user_id: usize,
}


impl MoveTimerUpdate {
    pub fn new(timer_value: usize, user_id: usize) -> MoveTimerUpdate {
        MoveTimerUpdate {
            name: "move_timer".to_string(),
            timer_value,
            user_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnChangeUpdate {
    name: String,
    pub turn_goes_to: usize
}

impl TurnChangeUpdate {
    pub fn new(turn_goes_to: usize) -> TurnChangeUpdate {
        TurnChangeUpdate {
            name: "turn_change".to_string(),
            turn_goes_to
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GameColorsUpdate<'a> {
    name: &'a str,
    pub room_id: &'a str,
    pub game: GameState,
}

impl GameColorsUpdate<'_> {
    pub fn new(room_id: &str, gs: GameState) -> GameColorsUpdate {
        GameColorsUpdate {
            name: "game_state",
            room_id,
            game: gs
        }
    }
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
    pub fn next_player(player: usize, total_players: usize) -> usize {
        return (player + 1) % total_players;
    }
    pub fn make_a_move(&mut self, path: Vec<(i32, i32)>, user_id: usize) -> std::result::Result<RoomUpdate, usize> {
        if let Some(gs) = self.game_state.as_mut() {
            let next = RoomHandle::next_player(self.active_player, self.players.len());
            let p = (path[0].0 as usize, path[0].1 as usize);
            if let Some(id) = gs.cones.get(&p) {
                if *id == user_id {
                    let update = gs.update_cones(&path, &user_id)
                        .map(|(path, game_finished)| {
                            self.active_player = next;
                            if game_finished {
                                self.winner = Some(user_id);
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
        Err(0)
    }
}

#[derive(Debug)]
pub enum  Message {
    Text(String),
    Event(String)
}

impl Message {
    pub fn event(evt: String) -> Message {
        Event(evt)
    }
}

#[derive(Debug)]
pub struct Player {
    pub user_id: usize,
    pub name: Option<String>,
    pub sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>,
    pub ready: bool,
    pub last_active: Instant
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

#[derive(Eq, PartialEq, Deserialize, Debug, Clone)]
pub enum UpdateRoomType {
    Start,
    Stop,
    ColorChange,
    Leave
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateRoomStateRequest {
    pub update_type: UpdateRoomType,
    pub new_color: Option<usize>
}

#[derive(Serialize, Debug)]
pub struct CreateRoomResponse {
    pub room: RoomDesc,
    pub url: String
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
#[derive(Debug)]
pub struct RoomNotFound;
#[derive(Debug)]
pub struct RoomFull;

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
    pub ready: bool
}

#[derive(Deserialize)]
pub struct RoomIdParameter {
    pub room_id: String
}

impl warp::reject::Reject for UserNotFound {}
impl warp::reject::Reject for RoomNotFound {}
impl warp::reject::Reject for RoomFull {}

impl PlayerDesc {
    pub fn from_player(p: &Player, color: usize) -> PlayerDesc {
        PlayerDesc {
            name: p.name.as_ref().cloned().or_else(|| { Some("Player".to_string()) }).unwrap(),
            color,
            user_id: p.user_id,
            ready: p.ready
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
