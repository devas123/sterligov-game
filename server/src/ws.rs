use futures::{FutureExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use tokio::sync::mpsc;
use warp::filters::ws::{Message, WebSocket};

use crate::{Player, RoomHandle, RoomList, User};

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

#[derive(Serialize, Debug)]
pub struct PlayerLeftUpdate {
    name: String,
    user_id: usize,
    room_id: String,
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
           room_id: String) -> PlayerLeftUpdate {
        return PlayerLeftUpdate {
            name: "player_left".to_string(),
            user_id,
            room_id
        };
    }
}

//TODO: we have thread per connection here, maybe use queues/dispatching thread
pub async fn client_connection(ws: WebSocket, id: String, user: User, rooms: RoomList, mut room: RoomHandle) {
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
    } else if room.game_started && !(room.game_state.is_some() && room.game_state.as_ref().unwrap().players_colors.get(&user.user_id).is_some()) {
        error!("Game is already started.");
    } else {
        let color = room.game_state.as_ref().map(|gs| {gs.players_colors.get(&user.user_id).cloned()}).flatten();
        let default_color = room.game_state.as_ref().map(|gs| {
            let mut color = 1;
            while color < 7 {
                info!("Getting default color: {:?}", gs.players_colors);
                for (pl, pl_col) in gs.players_colors.iter() {
                    info!("Getting default color: user: {}, user_color: {}, color: {}", *pl, *pl_col, color);
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
            user.user_id ,
            id.clone(),
            vec![],
            user.user_name.clone(),
            color.clone().or(Some(default_color.clone())).unwrap(),
        );
        let player = Player {
            sender: Some(player_sender),
            user_id: user.user_id,
            name: Some(user.user_name.clone()),
        };
        room.players.push(player);
        if let Some(gs) = room.game_state.as_mut() {
            gs.players_colors.insert(user.user_id, color.clone().or(Some(default_color.clone())).unwrap());
            if color.is_none() {
                if let Err(_) = gs.add_cones(default_color, user.user_id).map(|new_gs| {
                    update.player_cones = new_gs.get_cones(&user.user_id);
                    room.game_state = Some(new_gs);
                }) {
                    error!("Error while adding cones for player {}", user.user_id)
                }
            } else {
                update.player_cones = gs.get_cones(&user.user_id);
            }

        }
        info!("User with id {} connected to room {}", user.user_id, id);

        rooms.write().unwrap().insert(id.clone(), room);
        match rooms.clone().write().unwrap().get(&id) {
            None => {}
            Some(updated_room) => {
                send_update(&updated_room, &update);
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
        let mut remove_room = false;
        if let Some(new_room) = rooms.write().unwrap().get_mut(&id) {
            new_room.remove_player(user.user_id);
            remove_room = new_room.players.len() == 0;
            info!("User {} disconnected from room {}", user.user_id, id);
            let upd = PlayerLeftUpdate::new(
                user.user_id,
                new_room.room_id.clone()
            );
            send_update(new_room, &upd);
        }

        if remove_room {
            info!("Room {} has no members left, so it will be removed", id);
            rooms.write().unwrap().remove(&id);
            info!("Room {} removed", id);
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
