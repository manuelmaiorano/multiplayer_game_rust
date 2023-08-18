use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::{Result, Lobby, Lobbies, Player, ws::{self, broadcast_event}, game_state::{self, GameState, PlayerState}, time_util};
use warp::{http::StatusCode, reply::json, ws::Message, Reply};


#[derive(Deserialize, Serialize)]
pub struct CreateLobbyRequest {
    pub name: String,
    pub player_name: String
}

#[derive(Deserialize, Serialize)]
pub struct WsBody {
    lobby_name: String
}

#[derive(Deserialize, Serialize)]
pub struct  LobbyResponse {
    pub url: String,
    pub game_state: Option<GameState>
}

#[derive( Serialize, Deserialize, Debug)]
pub struct EnterLobby {
    pub name: String,
    pub player_name: String
}


pub async fn create_lobby(req: CreateLobbyRequest, lobbies: Lobbies) -> Result<impl Reply> {
    //let uuid =  Uuid::new_v4().as_simple().to_string();
    lobbies.write().await.insert(req.name.clone(), 
        Lobby { 
            players: HashMap::from_iter([(req.player_name.clone(), 
                Player {
                    lobby_name: Some(req.name.clone()),
                    sender: None
                }
            )]),
            game: GameState {
                players: HashMap::from_iter([(req.player_name.clone(),
                    PlayerState {
                        position: game_state::Vec2 { x: 0.0, y: 0.0 },
                        velocity: game_state::Vec2 { x: 0.0, y: 0.0 },
                        angle: 0.0,
                        name: req.player_name.clone(),
                        health: 100,
                        alive: true
                    }),
                ]),
                bullets: Vec::with_capacity(50),
                last_time: time_util::get_current_time(),
                actions: Vec::with_capacity(10)

            }
        });

    tokio::task::spawn(async move  {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            let mut locked = lobbies.write().await;
            let actions = locked.get_mut(&req.name).unwrap().game.update(time_util::get_current_time());
            
            actions.iter().for_each(|action| {
                match action {
                    game_state::Action::DeletePlayer(name) => {
                        // locked.get_mut(&req.name).unwrap().players.iter().for_each(|(_, player)| {
                        //     if let Some(sender) = &player.sender {
                        //         let _ = sender.send(Ok(Message::text(format!("{name} died"))));
                        //     }
                        // })
                        broadcast_event(locked.get_mut(&req.name).unwrap(), ws::GameEvent::Death(name.clone()));
                    },
                    _ => {}
                }
            })
        }
    });

    Ok(json(&LobbyResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", req.player_name),
        game_state: None
    }))
}

pub async fn delete_lobby(name: String, lobbies: Lobbies) -> Result<impl Reply> {
    lobbies.write().await.remove(&name);
    Ok(StatusCode::OK)
}

pub async fn enter_lobby(req: EnterLobby, lobbies: Lobbies) ->  Result<impl Reply> {
    let uuid =  Uuid::new_v4().as_simple().to_string();
    let mut locked = lobbies.write().await;
    locked.get_mut(&req.name).unwrap()
        .players.insert(uuid.clone(), 
            Player { 
                lobby_name: Some(req.name), 
                sender: None });
    locked.get_mut(&req.player_name).unwrap().game.add_player(&req.player_name, game_state::Vec2 { x: 0.0, y: 0.0 });

    Ok(json(&LobbyResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
        game_state: Some(locked.get_mut(&req.player_name).unwrap().game.clone())
    }))

}

pub async fn ws_handler(ws: warp::ws::Ws, body: WsBody, id: String, lobbies: Lobbies) ->  Result<impl Reply> {
    let player = lobbies.read().await.get(&body.lobby_name).unwrap().players.get(&id).cloned();
    match  player {
        Some(pl) => Ok(ws.on_upgrade(move |socket| ws::player_connection(socket, id, lobbies, body.lobby_name, pl))),
        None => Err(warp::reject::not_found())
    }
}