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

pub async fn client_connection(ws: WebSocket, id: String, user_id: usize, rooms: RoomList, mut room: RoomHandle) {
    let (player_ws_sender, mut player_ws_receiver) = ws.split();
    let (player_sender, player_receiver) = mpsc::unbounded_channel();

    tokio::task::spawn(player_receiver.forward(player_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    if room.players.len() > 5 {
        eprintln!("Room full");
    } else if room.players.iter().any(|x| { x.user_id == user_id }) {
        eprintln!("User already in the room.");
    } else {
        let player = Player {
            sender: Some(player_sender),
            user_id,
        };
        room.players.push(player);
        rooms.write().unwrap().insert(id.clone(), room);

        println!("{} connected", id);

        while let Some(result) = player_ws_receiver.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                    break;
                }
            };
            client_msg(&id, msg, &rooms).await;
        }

        rooms.write().unwrap().remove(&id);
        println!("{} disconnected", id);
    }
}

async fn client_msg(id: &str, msg: Message, rooms: &RoomList) {
    println!("received message from {}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        return;
    }

    let topics_req: TopicsRequest = match from_str(&message) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error while parsing message to topics request: {}", e);
            return;
        }
    };

    let mut locked = rooms.write().unwrap();
    if let Some(_v) = locked.get_mut(id) {
        println!("Move {} by user {}", topics_req.mv, id)
        // v.players = topics_req.;
    }
}
