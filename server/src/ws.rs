use futures::{Stream, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use warp::Error;
use warp::filters::sse::ServerSentEvent;

use crate::{Result, RoomHandle, User};
use crate::model::{Message, Player, RoomFull};
use tokio::sync::mpsc::UnboundedReceiver;
use std::time::Instant;
use core::fmt::Debug;

#[derive(Serialize, Debug)]
pub struct ChatMessage {
    name: String,
    by: String,
    message: Option<String>,
    ready: Option<bool>,
    user_id: usize,
}

#[derive(Deserialize, Debug)]
pub struct SendMessageRequest {
    pub message: Option<String>,
    pub set_ready: Option<bool>
}

impl ChatMessage {
    pub fn new(by: &str, user_id: usize, message: Option<String>, ready: Option<bool>) -> ChatMessage {
        return ChatMessage {
            name: "chat_message".to_string(),
            by: by.to_string(),
            message,
            ready,
            user_id,
        };
    }
}

#[derive(Serialize, Debug)]
pub struct PlayerJoinedUpdate {
    name: String,
    user_id: usize,
    room_id: String,
    player_cones: Vec<(usize, usize)>,
    player_name: String,
    player_color: usize,
    player_ready: bool
}

#[derive(Serialize, Debug)]
pub struct PlayerLeftUpdate {
    name: String,
    user_id: usize,
    player_color: usize,
    room_id: String,
    next_turn: usize,
    remove_cones: bool
}

impl PlayerJoinedUpdate {
    fn new(user_id: usize,
           room_id: String,
           player_cones: Vec<(usize, usize)>,
           player_name: String,
           player_color: usize,
           player_ready: bool) -> PlayerJoinedUpdate {
        return PlayerJoinedUpdate {
            name: "player_joined".to_string(),
            user_id,
            room_id,
            player_cones,
            player_name,
            player_color,
            player_ready
        };
    }
}

impl PlayerLeftUpdate {
    pub(crate) fn new(user_id: usize,
                      room_id: String,
                      next_turn: usize,
                      remove_cones: bool,
                      player_color: usize) -> PlayerLeftUpdate {
        return PlayerLeftUpdate {
            name: "player_left".to_string(),
            user_id,
            room_id,
            next_turn,
            remove_cones,
            player_color
        };
    }
}

pub fn client_connection(room_id: String, user: User, room: &mut RoomHandle) -> Result<impl Stream<Item=std::result::Result<impl ServerSentEvent + Send + 'static, Error>> + Send + 'static> {
    if room.players.len() > 5 {
        error!("Room full");
        Err(warp::reject::custom(RoomFull))
    } else if room.game_started && room.game_state.as_ref().filter(|gs| { gs.players_colors.get(&user.user_id).is_some() }).is_none() {
        error!("Game is already started.");
        Err(warp::reject::custom(RoomFull))
    } else {
        let color = room.game_state.as_ref().map(|gs| { gs.players_colors.get(&user.user_id).cloned() }).flatten();
        let default_color = room.game_state.as_ref().map(|gs| {
            let mut c = 1;
            while c < 7 {
                info!("Getting default color, current colors: {:?}", gs.players_colors);
                for (pl, pl_col) in gs.players_colors.iter() {
                    info!("Getting default color: user: {}, already used color: {}, testing color: {}", *pl, *pl_col, c);
                    if *pl_col == c {
                        c += 1;
                        continue;
                    }
                }
                break;
            }
            c
        }).or(Some(*&room.players.len() + 1)).unwrap();
        let player_color = color.unwrap_or_else(|| default_color);
        let mut update = PlayerJoinedUpdate::new(
            user.user_id,
            room_id.to_string(),
            vec![],
            user.user_name.clone(),
            player_color,
            false
        );
        let (player_sender, player_receiver) = mpsc::unbounded_channel();
        let result = if let Some(p) = room.players.iter_mut().find(|p| p.user_id == user.user_id) {
            p.sender = player_sender.clone();
            update.player_ready = p.ready;
            wrap(player_receiver)
        } else {
            let result= wrap(player_receiver);
            let player = Player {
                sender: player_sender.clone(),
                user_id: user.user_id,
                name: Some(user.user_name.clone()),
                last_active: Instant::now(),
                ready: false
            };
            room.players.push(player);
            if room.players.len() == 1 {
                room.created_by = user.user_id.clone();
            }
            result
        };
        info!("User with id {} connected to room {}", user.user_id, room_id);
        if let Some(gs) = room.game_state.as_mut() {
            if color.is_none() {
                if let Err(_) = gs.add_cones(user.user_id, player_color) {
                    error!("Error while adding cones for player {}", user.user_id)
                }
            }
            update.player_cones = gs.get_cones(&user.user_id);
        }
        send_update(&room, &update);
        result
    }
}

fn wrap(player_receiver: UnboundedReceiver<std::result::Result<Message, Error>>) -> Result<impl Stream<Item=std::result::Result<impl ServerSentEvent + Send + 'static, Error>> + Send + 'static> {
    Ok(player_receiver.map(|res| {
        res.map(|msg| {
            info!("Message: {:?}", msg);
            match msg {
                Message::Text(text) => {
                    warp::sse::data(text).into_a()
                }

                Message::Event(event) => {
                    warp::sse::event(event).into_b()
                }
            }
        })
    }))
}

pub fn send_update(rh: &RoomHandle, upd: &(impl Serialize + Debug)) {
    for p in &rh.players {
        match serde_json::ser::to_string(upd) {
            Ok(str) => {
                if let Err(e) = p.sender.send(Ok(Message::Text(str))) {
                    error!("Error while sending update  to players. {:?}, {:?}" , upd, e);

                }
            }
            Err(msg) => {
                error!("Error while serializing update {:?}, {:?}" , upd, msg);
            }
        }

    }
}