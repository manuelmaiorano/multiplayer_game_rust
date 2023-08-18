use std::{collections::HashMap, thread, time};
use std::sync::mpsc::{self, Sender, Receiver};
use macroquad::ui::{hash, root_ui, widgets};
use serde_json::{from_str, to_string};
use crate::handler::EnterLobby;
use crate::ws::{self, GameEvent, Commands};
use crate::{handler::{CreateLobbyRequest, LobbyResponse}, game_state::{self, GameState, PlayerState}, time_util};
use macroquad::prelude::*;
use game_state::Vec2;
use reqwest::blocking;
use tungstenite::{connect, Message};
use url::Url;

const URL: &str = "http://localhost:8000";

pub enum SetupMessage {
    CreateLobby {lobby_name: String, player_name: String},
    EnterLobby {lobby_name: String, player_name: String},
    LobbyEntered {url: String, game_state: Option<GameState>}
}


#[macroquad::main("MultiplayerGame")]
async fn main() {

    let (sender_setup, receiver_setup): (Sender<SetupMessage>, Receiver<SetupMessage>) = mpsc::channel();
    let (sender_lobby_enter, receiver_lobby_enter): (Sender<SetupMessage>, Receiver<SetupMessage>) = mpsc::channel();


    thread::spawn(move || {
        let msg = receiver_setup.recv().unwrap();
        let res = match msg {
            SetupMessage::CreateLobby { lobby_name, player_name } => {
                let client = blocking::Client::new();
                let res = client
                    .post(format!("{URL}/create_lobby"))
                    .json(&to_string(&CreateLobbyRequest {name: lobby_name.clone(), player_name: player_name.clone()}).unwrap())
                    .send()
                    .unwrap()
                    .text()
                    .unwrap();

                let res: LobbyResponse = from_str(&res).unwrap();
                Some(res)
            }, 
            SetupMessage::EnterLobby { lobby_name, player_name } => {
                let client = blocking::Client::new();
                let res = client
                    .post(format!("{URL}/register"))
                    .json(&to_string(&EnterLobby {
                        name: lobby_name.clone(), 
                        player_name: player_name.clone()
                    }).unwrap())
                    .send()
                    .unwrap()
                    .text()
                    .unwrap();

                let res: LobbyResponse = from_str(&res).unwrap();
                Some(res)
            },
            _ => {None}

        };
        if let Some(res) = res {
            sender_lobby_enter.send(SetupMessage::LobbyEntered { 
                url: res.url,
                game_state: res.game_state }).unwrap();
        }
    });

    let mut events_receiver: Option<_> = None;
    let mut action_sender: Option<_> = None;
    let mut in_lobby_menu = true;
    let mut game_state = GameState {
        players: HashMap::new(),
        bullets: Vec::with_capacity(50),
        last_time: time_util::get_current_time(),
        actions: Vec::with_capacity(10)
    };
    
    let mut vertical = 0.0;
    let mut horizontal = 0.0;
    loop {
        clear_background(WHITE);
        if in_lobby_menu {
            //show ui
            let mut lobby_name = String::new();
            let mut player_name = String::new();
            widgets::Window::new(hash!(), vec2(470., 50.), vec2(300., 300.))
                .label("lobby menu")
                .ui(&mut *root_ui(), |ui| {
                    ui.input_text(hash!(), "<- lobby name", &mut lobby_name);
                    ui.input_text(hash!(), "<- player name", &mut player_name);

                    if ui.button(None, "CREATE LOBBY") {
                        sender_setup.send(SetupMessage::CreateLobby { 
                            lobby_name: lobby_name.clone(), 
                            player_name: player_name.clone() }).unwrap();
                    }
                    if ui.button(None, "ENTER LOBBY") {
                        sender_setup.send(SetupMessage::EnterLobby  { 
                            lobby_name: lobby_name.clone(), 
                            player_name: player_name.clone() }).unwrap();
                    }
                });
            let msg = receiver_lobby_enter.try_recv().unwrap();
            match msg {
                SetupMessage::LobbyEntered { url, game_state: game } => {
                    let (receiver, sender) = spawn_comm_threads(url);
                    in_lobby_menu = false;
                    events_receiver = Some(receiver);
                    action_sender = Some(sender);
                    if let Some(game) = game {
                        game_state = game;
                    }
                },
                _ => {}
            }
        } else {
            let mut actions = Vec::with_capacity(10);
            if is_key_pressed(KeyCode::Space) {
                actions.push(Commands::Shoot);
            };

            let mut state_change = false;
            if is_key_pressed(KeyCode::A) {
                horizontal = -1.0;
                state_change = true;
            } else if is_key_pressed(KeyCode::D) {
                horizontal = 1.0;
                state_change = true;
            }
            if is_key_pressed(KeyCode::W) {
                vertical = -1.0;
                state_change = true;
            } else if is_key_pressed(KeyCode::S) {
                vertical = 1.0;
                state_change = true;
            }
            
            if state_change {
                let vel = vec2(horizontal, vertical).normalize() * 10.0;
                actions.push(Commands::UpdateVelocity { x: vel.x, y: vel.y });
            }

            while actions.len() > 0 {
                let action = actions.pop().unwrap();
                action_sender.as_ref().unwrap().send(action).unwrap();
            }

            game_state.update(time_util::get_current_time());
            game_state.players.iter().for_each(|(_, player)| {
                let x = player.position.x;
                let y = player.position.y;
                draw_circle(x, y, game_state::PLAYER_RADIUS_SIZE, RED);
            });
            game_state.bullets.iter().for_each(|bullet| {
                let x = bullet.position.x;
                let y = bullet.position.y;
                draw_circle(x, y, game_state::BULLET_RADIUS_SIZE, BLACK);

            });
            if let Ok(event) = events_receiver.as_ref().unwrap().try_recv() {
                match event {
                    GameEvent::Death(name) => {
                        game_state.kill_player(&name);
                    },
                    GameEvent::Shooting(name) => {
                        let player = game_state.players.get(&name).unwrap();
                        game_state.add_bullet(player.position.sum(&Vec2::with_angle(player.angle, 
                            game_state::PLAYER_RADIUS_SIZE + 1.0)), Vec2::with_angle(player.angle, game_state::BULLET_VEL));
                    },
                    GameEvent::UpdateAngle { angle, name } => {
                        let player = game_state.players.get_mut(&name).unwrap();
                        player.angle = angle;
                    },
                    GameEvent::UpdateVelocity { x, y, name } => {
                        let player = game_state.players.get_mut(&name).unwrap();
                        player.velocity.x = x;
                        player.velocity.y = y;
                    }
                }
            }
        }
        thread::sleep(time::Duration::from_millis(25));
        next_frame().await;
    }
}


fn spawn_comm_threads(url: String) -> (Receiver<ws::GameEvent>, Sender<ws::Commands>) {
    let (events_sender, events_receiver): (Sender<ws::GameEvent>, Receiver<ws::GameEvent>) = mpsc::channel();
    let (action_sender, action_receiver): (Sender<ws::Commands>, Receiver<ws::Commands>) = mpsc::channel();

    let (mut socket, response) = connect(Url::parse(&url).unwrap()).expect("cant connect");
    thread::spawn(move || {
        loop {
            let msg = socket.read_message().unwrap().to_text().unwrap().to_string();
            events_sender.send(from_str(&msg).unwrap()).unwrap();
            let msg = action_receiver.recv().unwrap();
            socket.write_message(Message::Text(to_string(&msg).unwrap())).unwrap();
        }
    });

    // thread::spawn(move || {
    //     loop {
    //         let msg = action_receiver.recv().unwrap();
    //         socket.write_message(Message::Text(to_string(&msg).unwrap())).unwrap();
    //     }
    // });

    (events_receiver, action_sender)
}