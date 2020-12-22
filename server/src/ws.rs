use futures::{FutureExt, StreamExt};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use warp::filters::ws::{Message, WebSocket};

use crate::{RoomHandle, RoomList, User};
use crate::model::Player;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage<'a> {
    name: &'a str,
    by: &'a str,
    message: &'a str,
    user_id: usize
}

impl ChatMessage<'_> {
    fn  new<'a>(by: &'a str, user_id: usize, message: &'a str) -> ChatMessage<'a> {
        return ChatMessage {
            name: "chat_message",
            by,
            message,
            user_id
        }
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
}

#[derive(Serialize, Debug)]
pub struct PlayerLeftUpdate {
    name: String,
    user_id: usize,
    room_id: String,
    next_turn: usize,
}

impl PlayerJoinedUpdate {
    fn new(user_id: usize,
           room_id: String,
           player_cones: Vec<(usize, usize)>,
           player_name: String,
           player_color: usize) -> PlayerJoinedUpdate {
        return PlayerJoinedUpdate {
            name: "player_joined".to_string(),
            user_id,
            room_id,
            player_cones,
            player_name,
            player_color,
        };
    }
}

impl PlayerLeftUpdate {
    fn new(user_id: usize,
           room_id: String,
           next_turn: usize) -> PlayerLeftUpdate {
        return PlayerLeftUpdate {
            name: "player_left".to_string(),
            user_id,
            room_id,
            next_turn,
        };
    }
}

//TODO: we have thread per connection here, maybe use queues/dispatching thread
pub async fn client_connection(ws: WebSocket, room_id: String, user: User, rooms: RoomList, mut room: RoomHandle) {
    let (player_ws_sender, mut player_ws_receiver) = ws.split();
    let (player_sender, player_receiver) = mpsc::unbounded_channel();

    tokio::task::spawn(player_receiver.forward(player_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("Error sending websocket msg: {}", e);
        }
    }));
    if room.players.len() > 5 {
        error!("Room full");
    } else if room.players.iter().any(|x| { x.user_id == user.user_id }) {
        info!("User already in the room.");
    } else if room.game_started && room.game_state.as_ref().filter(|gs| { gs.players_colors.get(&user.user_id).is_some() }).is_none() {
        error!("Game is already started.");
    } else {
        let color = room.game_state.as_ref().map(|gs| { gs.players_colors.get(&user.user_id).cloned() }).flatten();
        let default_color = room.game_state.as_ref().map(|gs| {
            let mut color = 1;
            while color < 7 {
                info!("Getting default color, current colors: {:?}", gs.players_colors);
                for (pl, pl_col) in gs.players_colors.iter() {
                    info!("Getting default color: user: {}, already used color: {}, testing color: {}", *pl, *pl_col, color);
                    if *pl_col == color {
                        color += 1;
                        continue;
                    }
                }
                break;
            }
            color
        }).or(Some(*&room.players.len() + 1)).unwrap();
        let mut update = PlayerJoinedUpdate::new(
            user.user_id,
            room_id.to_string(),
            vec![],
            user.user_name.clone(),
            color.clone().or(Some(default_color.clone())).unwrap(),
        );
        let player = Player {
            sender: Some(player_sender.clone()),
            user_id: user.user_id,
            name: Some(user.user_name.clone()),
        };
        room.players.push(player);
        if room.players.len() == 1 {
            room.created_by = user.user_id.clone();
        }
        if let Some(gs) = room.game_state.as_mut() {
            gs.players_colors.insert(user.user_id, color.clone().or(Some(default_color.clone())).unwrap());
            if color.is_none() {
                if let Err(_) = gs.add_cones(default_color) {
                    error!("Error while adding cones for player {}", user.user_id)
                }
            }
            update.player_cones = gs.get_cones(&user.user_id);
        }
        info!("User with id {} connected to room {}", user.user_id, room_id);

        rooms.write().unwrap().insert(room_id.clone(), room);
        match rooms.clone().write().unwrap().get(room_id.as_str()) {
            None => {}
            Some(updated_room) => {
                send_update(&updated_room, &update);
            }
        }

        while let Some(result) = player_ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Error receiving ws message for id: {}): {}", room_id, e);
                    break;
                }
            };
            client_msg(room_id.as_str(), &player_sender, msg, &rooms, &user).await;
        }
        match rooms.try_write() {
            Ok(mut lock) => {
                let mut remove_room = false;
                if let Some(new_room) = lock.get_mut(room_id.as_str()) {
                    new_room.remove_player(user.user_id);
                    remove_room = new_room.players.len() == 0 && new_room.game_finished;
                    let new_turn = if new_room.players.len() == 0 { 0 } else { new_room.active_player % new_room.players.len() };
                    new_room.active_player = new_turn;
                    info!("User {} disconnected from room {}", user.user_id, room_id);
                    let upd = PlayerLeftUpdate::new(
                        user.user_id,
                        new_room.room_id.clone(),
                        new_turn
                    );
                    send_update(new_room, &upd);
                }
                if remove_room {
                    lock.remove(&room_id);
                }
            }
            Err(e) => {
                error!("Error while acquiring the lock. {:?}", e)
            }
        }
    }
}

pub fn send_update(rh: &RoomHandle, upd: &impl Serialize) {
    for p in &rh.players {
        if let Some(sender) = p.sender.as_ref() {
            if let Err(msg) = sender.send(Ok(serde_json::ser::to_string(&upd).map(Message::text).unwrap())) {
                error!("Error while sending update to players. {:?}", msg)
            }
        }
    }
}

async fn client_msg(room_id: &str, sender: &mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>, msg: Message, rooms: &RoomList, user: &User) {
    debug!("Received message from {}: {:?}", room_id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return
    };

    if message == "ping" || message == "ping\n" {
        if let Err(err) = sender.send(Ok(Message::text("pong"))) {
            error!("Error while sending pong message. {:?}", err)
        }
        return;
    }

    let chat_message = ChatMessage::new(user.user_name.as_str(), user.user_id, message);

    let mut locked = rooms.write().unwrap();
    if let Some(room) = locked.get_mut(room_id) {
        debug!("Message {} by user {}", chat_message.message, room_id);
        send_update(room, &chat_message);
        // v.players = topics_req.;
    }
}
