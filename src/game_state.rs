use std::collections::HashMap;
use crate::time_util;
use serde::{Serialize, Deserialize};

pub const BULLET_VEL: f32 = 20.0;
pub const PLAYER_RADIUS_SIZE: f32 = 10.0;
pub const BULLET_RADIUS_SIZE: f32 = 3.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32
}

impl Vec2 {
    const DEG2RAD: f32 = std::f32::consts::PI/180.0;

    pub fn sum(&self, other: &Self) -> Self {
        Vec2 { x: self.x + other.x, y: self.y + other.y}
    }

    pub fn with_angle(angle: f32, len: f32) -> Self {
        Vec2 { x: len * (angle * Vec2::DEG2RAD).cos(), y: len * (angle * Vec2::DEG2RAD).cos() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub name: String,
    pub position: Vec2,
    pub velocity: Vec2, 
    pub angle: f32,
    pub health: i32,
    pub alive: bool
}

impl PlayerState {
    pub fn update(&mut self, delta_time: f32, bullets: &Vec<BulletState>) -> Option<Action> {
        self.position.x += self.velocity.x * delta_time;
        self.position.y += self.velocity.y * delta_time;
        if bullets.iter().any(|state: &BulletState| {
            (state.position.x - self.position.x).powi(2) +
            (state.position.y - self.position.y).powi(2) < 
            PLAYER_RADIUS_SIZE.powi(2)
        }) {
            return  Some(Action::DeletePlayer(self.name.clone()));
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletState {
    pub position: Vec2,
    velocity: Vec2,
    lifetime: f32,
    time: f32,
    index: usize
}

impl  BulletState {
    pub fn update(&mut self, delta_time: f32) -> Option<Action>{
        self.position.x += self.velocity.x * delta_time;
        self.position.y += self.velocity.y * delta_time;
        self.time += delta_time;
        if self.time >= self.lifetime {
            return Some(Action::DeleteBullet(self.index));
        }
        None
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    DeletePlayer(String),
    DeleteBullet(usize)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub players: HashMap<String, PlayerState>,
    pub bullets: Vec<BulletState>,
    pub last_time: f32, 
    pub actions: Vec<Action>
}

impl GameState {
    pub fn update(&mut self, current_time: f32) -> Vec<Action>{
        let delta_time = current_time - self.last_time;
        self.last_time = current_time;

        while !self.actions.is_empty() {
            match self.actions.pop().unwrap() {
                Action::DeleteBullet(idx) => self.remove_bullet(idx),
                Action::DeletePlayer(name) => self.kill_player(&name)
            }
        }

        self.players.iter_mut().for_each(|(_, state)| {
            if let Some(act) = state.update(delta_time, &self.bullets) {
                self.actions.push(act);
            }
        });

        self.bullets.iter_mut().for_each(|state: &mut BulletState| {
            if let Some(act) = state.update(delta_time) {
                self.actions.push(act);
            }
        });
        
        self.actions.clone()
    }

    pub fn kill_player(&mut self, name: &str) {
        self.players.remove(name).unwrap();
    }

    pub fn remove_bullet(&mut self, index: usize) {
        self.bullets.remove(index);
    }

    pub fn add_bullet(&mut self, pos: Vec2, vel: Vec2){
        self.bullets.push(BulletState { position: pos, velocity: vel, lifetime: 10.0, 
            time: time_util::get_current_time(), index: self.bullets.len() })
    }

    pub fn add_player(&mut self, name: &str, pos: Vec2){
        self.players.insert(name.to_string(), PlayerState { name: name.to_string(), position: pos, velocity: Vec2 { x: 0.0, y: 0.0 }, 
            angle: 0.0, health: 100, alive: true}).unwrap();
    }
}