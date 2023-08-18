use futures::{FutureExt, StreamExt};
use warp::ws::{Message, WebSocket};
use crate::{Lobbies, Lobby, Player, game_state::{Vec2, self}};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Deserialize, Serialize, Debug)]
pub enum Commands {
    UpdateVelocity {x: f32, y: f32},
    Shoot,
    UpdateAngle(f32)
}

#[derive(Deserialize, Serialize)]
pub enum GameEvent {
    Shooting(String),
    UpdateVelocity {x: f32, y: f32, name: String},
    UpdateAngle {angle: f32, name: String},
    Death(String)
}


pub async fn player_connection(ws: WebSocket, id: String, lobbies: Lobbies, lobby_name: String, mut player: Player) {
    let (ws_sender, mut ws_receiver) = ws.split();
    let (player_sender, player_rcv) = mpsc::unbounded_channel();

    let player_rcv = UnboundedReceiverStream::new(player_rcv);
    tokio::task::spawn(player_rcv.forward(ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending ws msg: {}", e);
        }
    }));

    player.sender = Some(player_sender);
    lobbies.write().await.get_mut(&lobby_name).unwrap().players.insert(id.clone(), player);

    while let Some(result) = ws_receiver.next().await {
        let msg = match  result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        player_msg(&id, msg, &lobbies, &lobby_name).await;
    }

    lobbies.write().await.get_mut(&lobby_name).unwrap().players.remove(&id).unwrap();


}

async fn player_msg(id: &str, msg: Message, lobbies: &Lobbies, lobby_name: &str) {
    println!("received message from {}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        return;
    }

    let command: Commands = match  from_str(&message) {
        Ok(v) => v,
        Err(e)  => {
            eprintln!("error while parsing message to topics request: {}", e);
            return;
        }
    };

    match command {
        Commands::Shoot => {
            let mut locked = lobbies.write().await;
            let game = &mut locked.get_mut(lobby_name).unwrap().game;
            let player = game.players.get(id).unwrap();

            game.add_bullet(player.position.sum(&Vec2::with_angle(player.angle, 
                game_state::PLAYER_RADIUS_SIZE + 1.0)), Vec2::with_angle(player.angle, game_state::BULLET_VEL));

            // locked.get(lobby_name).unwrap().players.iter().for_each(|(_, player)| {
            //     if let Some(sender) = &player.sender {
            //         let _ = sender.send(Ok(Message::text(to_string(&GameEvent::Shooting(id.to_string())).unwrap())));
            //     }
            // });
            broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::Shooting(id.to_string()));

        },
        Commands::UpdateAngle(angle) => {
            let mut locked = lobbies.write().await;
            let game = &mut locked.get_mut(lobby_name).unwrap().game;
            let player = &mut game.players.get_mut(id).unwrap();

            player.angle = angle;

            broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::UpdateAngle { angle: angle, name: id.to_string() });

        }, 
        Commands::UpdateVelocity { x, y } => {
            let mut locked = lobbies.write().await;
            let game = &mut locked.get_mut(lobby_name).unwrap().game;
            let player = &mut game.players.get_mut(id).unwrap();

            player.velocity.x = x;
            player.velocity.y = y;

            broadcast_event(locked.get(lobby_name).unwrap(), GameEvent::UpdateVelocity { x: x, y: y, name: id.to_string() });

        }
    }
}

pub fn broadcast_event(lobby: &Lobby, event: GameEvent) {
    lobby.players.iter().for_each(|(_, player)| {
        if let Some(sender) = &player.sender {
            let _ = sender.send(Ok(Message::text(to_string(&event).unwrap())));
        }
    });
}