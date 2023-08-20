use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::{Result, Lobby, Lobbies, Player, ws::{self}, game_state::{self, GameState, GameEvent, PlayerState, Vec2}, time_util, SetupMessage, Channels};
use warp::{http::StatusCode, reply::json, ws::Message, Reply};
use tokio::sync::{mpsc, broadcast};

#[derive(Deserialize, Serialize)]
pub struct CreateLobbyRequest {
    pub name: String,
    pub player_name: String
}

#[derive(Deserialize, Serialize, Debug)]
pub struct WsBody {
    lobby_name: String
}

#[derive(Deserialize, Serialize, Debug)]
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
    println!("received: {:?}", req.name);
    let lobby_name = req.name.clone();
    let player_name = req.player_name.clone();
    let mut game_state = GameState {
        players: HashMap::from_iter([(req.player_name.clone(),
            PlayerState {
                position: game_state::Vec2 { x: 100.0, y: 100.0 },
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

    };
    let (setup_tx, mut setup_rx) = mpsc::unbounded_channel();
    let (ch_tx, mut ch_rx) = mpsc::unbounded_channel();

    lobbies.write().await.insert(lobby_name.clone(), Lobby { 
        game_setup_sender: setup_tx, 
        game_ch_receiver: ch_rx });

    let initial_state = game_state.clone();

    tokio::task::spawn(async move  {
        
        let setup = setup_rx.recv().await.unwrap();
        let (br_tx, mut event_rx, event_tx) = match setup {
            SetupMessage::AddPlayer(name) => {
                let (br_tx, mut br_rx) = broadcast::channel(20);
                let (event_tx, mut event_rx): (mpsc::UnboundedSender<GameEvent>, mpsc::UnboundedReceiver<GameEvent>) = mpsc::unbounded_channel();
                // ch_tx.send(Channels {
                //     broadcast_receiver: Some(br_tx.subscribe()),
                //     event_sender: Some(event_tx.clone())
                // }).unwrap();
                (Some(br_tx), Some(event_rx), Some(event_tx))
            },
            _ => {(None, None, None)}
        };

        println!("started game loop");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
            let events = game_state.update(time_util::get_current_time());
            
            events.iter().for_each(|event| {
                br_tx.as_ref().unwrap().send(event.clone()).unwrap();
            });
            if let Ok(event) = event_rx.as_mut().unwrap().try_recv() {
                println!("received event: {:?}", event);
                br_tx.as_ref().unwrap().send(event.clone()).unwrap();
                game_state.react_to_event(event);
            }
            if let Ok(setup_msg) = setup_rx.try_recv() {
                match setup_msg {
                    SetupMessage::AddPlayer(name) => {
                        game_state.add_player(&name, Vec2 {x: 0.0, y: 0.0});
                        println!("added new player: {:?}", name);
                        br_tx.as_ref().unwrap().send(GameEvent::AddPlayer { x: 0.0, y: 0.0, name: name }).unwrap();
                        br_tx.as_ref().unwrap().send(GameEvent::GameStateSync(game_state.clone())).unwrap();
                    },
                    SetupMessage::GetChannels => {
                        ch_tx.send(Channels {
                            broadcast_receiver: Some(br_tx.as_ref().unwrap().subscribe()),
                            event_sender: Some(event_tx.as_ref().unwrap().clone())
                        }).unwrap();
                        br_tx.as_ref().unwrap().send(GameEvent::GameStateSync(game_state.clone())).unwrap();
                    }
                }
                
            }
        };
    });

    let msg = LobbyResponse {
        url: format!("ws://localhost:8000/ws/{}/{}", lobby_name, req.player_name),
        game_state: Some(initial_state)
    };
    println!("sent : {:?}", msg);
    Ok(json(&msg))
}

pub async fn delete_lobby(name: String, lobbies: Lobbies) -> Result<impl Reply> {
    lobbies.write().await.remove(&name);
    Ok(StatusCode::OK)
}

pub async fn enter_lobby(req: EnterLobby, lobbies: Lobbies) ->  Result<impl Reply> {
    let uuid =  Uuid::new_v4().as_simple().to_string();
    let locked = lobbies.read().await;
    locked.get(&req.name).unwrap();

    Ok(json(&LobbyResponse {
        url: format!("ws://localhost:8000/ws/{}/{}", req.name, req.player_name),
        game_state: None
    }))

}

pub async fn ws_handler(ws: warp::ws::Ws, lobby_name: String, id: String, lobbies: Lobbies) ->  Result<impl Reply> {
    println!("tryng to ws connect to: {:?}", lobby_name);
    if lobbies.read().await.contains_key(&lobby_name) {
        Ok(ws.on_upgrade(move |socket| ws::player_connection(socket, lobbies, lobby_name, id)))
    } else {
        Err(warp::reject::not_found())
    }
}

