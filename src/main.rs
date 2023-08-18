use warp::{ws::Message, Filter, Rejection};
use std::{convert::Infallible, collections::HashMap};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

mod handler;
mod ws;
mod game_state;
mod time_util;
mod client_loop;

type Result<T> = std::result::Result<T, Rejection>;
type Lobbies = Arc<RwLock<HashMap<String, Lobby>>>;

#[derive(Debug, Clone)]
pub struct Player{
    pub lobby_name: Option<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>

}

pub struct Lobby {
    pub players: HashMap<String, Player>,
    pub game: game_state::GameState
}

#[tokio::main]
async fn main() {
    let lobbies: Lobbies = Arc::new(RwLock::new(HashMap::new()));
    let lobby_creation = warp::path("create_lobby");
    let lobby_routes = lobby_creation
        .and(warp::post())
        .and(warp::body::json())
        .and(whith_lobbies(lobbies.clone()))
        .and_then(handler::create_lobby)
        .or(lobby_creation
                .and(warp::delete())
                .and(warp::path::param())
                .and(whith_lobbies(lobbies.clone()))
                .and_then(handler::delete_lobby));

    let enter_lobby = warp::path("register")
            .and(warp::post())
            .and(warp::body::json())
            .and(whith_lobbies(lobbies.clone()))
            .and_then(handler::enter_lobby);

    let ws_route = warp::path("ws")
            .and(warp::ws())
            .and(warp::body::json())
            .and(warp::path::param())
            .and(whith_lobbies(lobbies.clone()))
            .and_then(handler::ws_handler);

    let routes = lobby_routes
            .or(enter_lobby)
            .or(ws_route)
            .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;


}

fn whith_lobbies(lobbies: Lobbies) -> impl Filter<Extract = (Lobbies,), Error = Infallible> + Clone {
    warp::any().map(move || lobbies.clone())
}