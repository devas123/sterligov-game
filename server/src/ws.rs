use futures::{Stream, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use warp::Error;
use warp::filters::sse::ServerSentEvent;

use crate::{Result, RoomHandle, User};
use crate::model::{Message, Player, RoomFull};
use serde::export::fmt::Debug;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Serialize, Debug)]
pub struct ChatMessage {
    name: String,
    by: String,
    message: String,
    user_id: usize,
}

#[derive(Deserialize, Debug)]
pub struct SendMessageRequest {
    pub message: String,
}

impl ChatMessage {
    pub fn new(by: &str, user_id: usize, message: &str) -> ChatMessage {
        return ChatMessage {
            name: "chat_message".to_string(),
            by: by.to_string(),
            message: message.to_string(),
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
    pub(crate) fn new(user_id: usize,
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
pub fn client_connection(room_id: String, user: User, room: &mut RoomHandle) -> Result<impl Stream<Item=std::result::Result<impl ServerSentEvent + Send + 'static, Error>> + Send + 'static> {
    // tokio::task::spawn(player_receiver.forward(player_sender).map(|result| {
    //     if let Err(e) = result {
    //         error!("Error sending sse msg: {}", e);
    //     }
    // }));
    if room.players.len() > 5 {
        error!("Room full");
        Err(warp::reject::custom(RoomFull))
    } else if room.game_started && room.game_state.as_ref().filter(|gs| { gs.players_colors.get(&user.user_id).is_some() }).is_none() {
        error!("Game is already started.");
        Err(warp::reject::custom(RoomFull))
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
        let (player_sender, player_receiver) = mpsc::unbounded_channel();
        let result = if let Some(p) = room.players.iter_mut().find(|p| p.user_id == user.user_id) {
            p.sender = player_sender.clone();
            wrap(player_receiver)
        } else {
            let result= wrap(player_receiver);
            let player = Player {
                sender: player_sender.clone(),
                user_id: user.user_id,
                name: Some(user.user_name.clone()),
            };
            room.players.push(player);
            if room.players.len() == 1 {
                room.created_by = user.user_id.clone();
            }
            result
        };
        info!("User with id {} connected to room {}", user.user_id, room_id);
        if let Some(gs) = room.game_state.as_mut() {
            gs.players_colors.insert(user.user_id, color.clone().or(Some(default_color.clone())).unwrap());
            if color.is_none() {
                if let Err(_) = gs.add_cones(default_color) {
                    error!("Error while adding cones for player {}", user.user_id)
                }
            }
            update.player_cones = gs.get_cones(&user.user_id);
        }
        send_update(&room, &update);
        result

        // while let Some(result) = player_ws_receiver.next().await {
        //     let msg = match result {
        //         Ok(msg) => msg,
        //         Err(e) => {
        //             error!("Error receiving ws message for id: {}): {}", room_id, e);
        //             break;
        //         }
        //     };
        //     client_msg(room_id.as_str(), &player_sender, msg, &rooms, &user).await;
        // }
        // match rooms.try_write() {
        //     Ok(mut lock) => {
        //         let mut remove_room = false;
        //         if let Some(new_room) = lock.get_mut(room_id.as_str()) {
        //             new_room.remove_player(user.user_id);
        //             remove_room = new_room.players.len() == 0 && new_room.game_finished;
        //             let new_turn = if new_room.players.len() == 0 { 0 } else { new_room.active_player % new_room.players.len() };
        //             new_room.active_player = new_turn;
        //             info!("User {} disconnected from room {}", user.user_id, room_id);
        //             let upd = PlayerLeftUpdate::new(
        //                 user.user_id,
        //                 new_room.room_id.clone(),
        //                 new_turn
        //             );
        //             send_update(new_room, &upd);
        //         }
        //         if remove_room {
        //             lock.remove(&room_id);
        //         }
        //     }
        //     Err(e) => {
        //         error!("Error while acquiring the lock. {:?}", e)
        //     }
        // }
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

/*async fn client_msg(room_id: &str, sender: &mpsc::UnboundedSender<std::result::Result<T, warp::Error>>, msg: Message, rooms: &RoomList, user: &User) {
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
*/