use futures::{FutureExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tokio::sync::mpsc;
use warp::filters::ws::{Message, WebSocket};

use crate::{Player, RoomHandle, RoomList};

#[derive(Deserialize, Debug)]
pub struct TopicsRequest {
    mv: String
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

//TODO: we have thread per connection here, maybe use queues/dispatching thread
pub async fn client_connection(ws: WebSocket, id: String, user_id: usize, rooms: RoomList, mut room: RoomHandle) {
    let (player_ws_sender, mut player_ws_receiver) = ws.split();
    let (player_sender, player_receiver) = mpsc::unbounded_channel();

    tokio::task::spawn(player_receiver.forward(player_ws_sender).map(|result| {
        if let Err(e) = result {
            error!("Error sending websocket msg: {}", e);
        }
    }));
    if room.players.len() > 5 {
        error!("Room full");
    } else if room.players.iter().any(|x| { x.user_id == user_id }) {
        info!("User already in the room.");
    } else if room.game_started {
        error!("Game is already started.");
    } else {
        let mut update = PlayerJoinedUpdate::new(
            user_id,
            id.clone(),
            vec![],
            "Player".to_ascii_lowercase(),
            *&room.players.len() + 1,
        );
        let player = Player {
            sender: Some(player_sender),
            user_id,
            color: Some(room.players.len() + 1),
            name: None,
        };
        room.players.push(player);
        if let Some(gs) = &room.game_state {
            let new_gs = gs.add_cones_for_player(&room.players.len() - 1, user_id).unwrap();
            update.player_cones = new_gs.get_cones(user_id);
            room.game_state = Some(new_gs);
        }
        info!("User with id {} connected to room {}", user_id, id);

        rooms.write().unwrap().insert(id.clone(), room);
        match rooms.clone().write().unwrap().get(&id) {
            None => {}
            Some(updated_room) => {
                for p in &updated_room.players {
                    if p.sender.is_some() {
                        if let Err(msg) = p.sender.as_ref().unwrap().send(Ok(serde_json::ser::to_string(&update).map(Message::text).unwrap())) {
                            error!("Error while sending update to players. {:?}", msg)
                        }
                    }
                }
            }
        }

        while let Some(result) = player_ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Error receiving ws message for id: {}): {}", id.clone(), e);
                    break;
                }
            };
            client_msg(&id, msg, &rooms).await;
        }

        let new_room = rooms.write().unwrap().get(&id).unwrap().remove_player(user_id);
        match new_room {
            Some(rh) => {
                info!("User {} disconnected from room {}", user_id, id);
                rooms.write().unwrap().insert(id.clone(), rh);
            }
            None => {
                info!("Room {} has no members left, so it is removed", id);
                rooms.write().unwrap().remove(&id);
            }
        }
    }
}

async fn client_msg(id: &str, msg: Message, rooms: &RoomList) {
    info!("Received message from {}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return
    };

    if message == "ping" || message == "ping\n" {
        return;
    }

    let topics_req: TopicsRequest = match from_str(&message) {
        Ok(v) => v,
        Err(e) => {
            error!("Error while parsing message to topics request: {}", e);
            return;
        }
    };

    let mut locked = rooms.write().unwrap();
    if let Some(_v) = locked.get_mut(id) {
        info!("Move {} by user {}", topics_req.mv, id)
        // v.players = topics_req.;
    }
}
