use futures::{SinkExt, FutureExt, StreamExt};
use warp::ws::{Message, WebSocket};
use crate::{Lobbies, Lobby, Player, game_state::{Vec2, self}};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio::sync::{mpsc};

#[derive(Deserialize, Serialize, Debug)]
pub enum Commands {
    UpdateVelocity {x: f32, y: f32},
    Shoot,
    UpdateAngle(f32)
}


pub async fn player_connection(ws: WebSocket, lobbies: Lobbies, lobby_name: String, player_name: String) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    //let (player_sender, player_rcv): (mpsc::UnboundedSender<GameEvent>, mpsc::UnboundedReceiver<GameEvent>) = mpsc::unbounded_channel();

    lobbies.read().await.get(&lobby_name).unwrap().game_setup_sender
        .send(crate::SetupMessage::AddPlayer(player_name.clone())).unwrap();
    println!("sent add player message");
    lobbies.read().await.get(&lobby_name).unwrap().game_setup_sender
        .send(crate::SetupMessage::GetChannels).unwrap();
    let mut channels = lobbies.write().await.get_mut(&lobby_name).expect("not found lobby name").game_ch_receiver.recv().await
        .expect("could not receive channels");
    println!("{:?} got the channels", player_name.clone());
    // tokio::task::spawn(player_rcv.forward(ws_sender).map(|result| {
    //     if let Err(e) = result {
    //         eprintln!("error sending ws msg: {}", e);
    //     }
    // }));
    tokio::task::spawn(async move {
        let mut br_rx = channels.broadcast_receiver.take().unwrap();
        loop {
            let event = br_rx.recv().await.unwrap();
            ws_sender.send(Message::text(to_string(&event).unwrap())).await.unwrap();
        }
    });

    println!("listening");
    while let Some(result) = ws_receiver.next().await {
        println!("received ws message");
        let msg = match  result {
            Ok(msg) => msg,
            Err(e) => {
                println!("error receiving ws message for id: {}): {}", player_name.clone(), e);
                //println!("error receiving ws message for id");
                break;
            }
        };
        channels.event_sender.as_ref().unwrap().send(from_str(&msg.to_str().unwrap()).unwrap()).unwrap();
        //println!("received message from {}: {:?}", id, msg);
        //player_msg(&id, msg, &lobbies, &lobby_name).await;
    }

    //lobbies.write().await.get_mut(&lobby_name).unwrap().players.remove(&id).unwrap();


}

// async fn player_msg(id: &str, msg: Message, lobbies: &Lobbies, lobby_name: &str) {
//     println!("received message from {}: {:?}", id, msg);
//     let message = match msg.to_str() {
//         Ok(v) => v,
//         Err(_) => return,
//     };

//     if message == "ping" || message == "ping\n" {
//         return;
//     }

//     let command: Commands = match  from_str(&message) {
//         Ok(v) => v,
//         Err(e)  => {
//             eprintln!("error while parsing message to topics request: {}", e);
//             return;
//         }
//     };

//     match command {
//         Commands::Shoot => {
//             let mut locked = lobbies.write().await;
//             let game = &mut locked.get_mut(lobby_name).unwrap().game;
//             let player = game.players.get(id).unwrap();

//             game.add_bullet(player.position.sum(&Vec2::with_angle(player.angle, 
//                 game_state::PLAYER_RADIUS_SIZE + 1.0)), Vec2::with_angle(player.angle, game_state::BULLET_VEL));

//             // locked.get(lobby_name).unwrap().players.iter().for_each(|(_, player)| {
//             //     if let Some(sender) = &player.sender {
//             //         let _ = sender.send(Ok(Message::text(to_string(&GameEvent::Shooting(id.to_string())).unwrap())));
//             //     }
//             // });
//             broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::Shooting(id.to_string()));

//         },
//         Commands::UpdateAngle(angle) => {
//             let mut locked = lobbies.write().await;
//             let game = &mut locked.get_mut(lobby_name).unwrap().game;
//             let player = &mut game.players.get_mut(id).unwrap();

//             player.angle = angle;

//             broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::UpdateAngle { angle: angle, name: id.to_string() });

//         }, 
//         Commands::UpdateVelocity { x, y } => {
//             let mut locked = lobbies.write().await;
//             let game = &mut locked.get_mut(lobby_name).unwrap().game;
//             let player = &mut game.players.get_mut(id).unwrap();

//             player.velocity.x = x;
//             player.velocity.y = y;

//             broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::UpdateVelocity { x: x, y: y, name: id.to_string() });

//         }
//     }
// }

// pub fn broadcast_event(lobby: &Lobby, event: GameEvent) {
//     println!("broadcasting: {:?}", event);
//     lobby.players.iter().for_each(|(_, player)| {
//         if let Some(sender) = &player.sender {
//             let _ = sender.send(Ok(Message::text(to_string(&event).unwrap())));
//         }
//     });
// }
