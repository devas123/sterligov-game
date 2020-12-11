use tokio::sync::mpsc;
use warp::filters::ws::{WebSocket, Message};
use serde_json::from_str;
use serde::Deserialize;
use futures::{StreamExt, FutureExt};


use crate::{RoomHandle, RoomList, Player};

#[derive(Deserialize, Debug)]
pub struct TopicsRequest {
    mv: String
}

//TODO: we have thread per connection here, maybe use queues/dispatching thread
pub async fn client_connection(ws: WebSocket, id: String, user_id: usize, rooms: RoomList, mut room: RoomHandle) {
    let (player_ws_sender, mut player_ws_receiver) = ws.split();
    let (player_sender, player_receiver) = mpsc::unbounded_channel();

    tokio::task::spawn(player_receiver.forward(player_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("Error sending websocket msg: {}", e);
        }
    }));

    if room.players.len() > 5 {
        eprintln!("Room full");
    } else if room.players.iter().any(|x| { x.user_id == user_id }) {
        println!("User already in the room.");
    } else if room.game_started {
        eprintln!("Game is already started.");
    } else {
        let player = Player {
            sender: Some(player_sender),
            user_id,
            color: Some(room.players.len() + 1),
            name: None
        };
        room.players.push(player);
        rooms.write().unwrap().insert(id.clone(), room);

        println!("User with id {} connected to room {}", user_id, id);

        while let Some(result) = player_ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("Error receiving ws message for id: {}): {}", id.clone(), e);
                    break;
                }
            };
            client_msg(&id, msg, &rooms).await;
        }

        let new_room = rooms.write().unwrap().get(&id).unwrap().remove_player(user_id);
        match new_room {
            Some(rh) => {
                println!("User {} disconnected from room {}", user_id, id);
                rooms.write().unwrap().insert(id.clone(), rh);
            }
            None => {
                println!("Room {} has no members left, so it is removed", id);
                rooms.write().unwrap().remove(&id);
            }
        }
    }
}

async fn client_msg(id: &str, msg: Message, rooms: &RoomList) {
    println!("Received message from {}: {:?}", id, msg);
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
            eprintln!("Error while parsing message to topics request: {}", e);
            return;
        }
    };

    let mut locked = rooms.write().unwrap();
    if let Some(_v) = locked.get_mut(id) {
        println!("Move {} by user {}", topics_req.mv, id)
        // v.players = topics_req.;
    }
}
