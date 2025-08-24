use serde::{Deserialize, Serialize};

pub const EARTH_RADIUS_KM: f32 = 6371.0;
pub const NEIGHBORS: usize = 20;

#[derive(Serialize, Deserialize, Debug)]
pub struct TripletLossTrainingLine {
    pub anchor: Coord,
    pub positive: Coord,
    pub negative: Coord,
    pub positive_time: f32,
    pub negative_time: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Coord {
    pub lat: f32,
    pub lon: f32,
}