
pub const EARTH_RADIUS_KM: f32 = 6371.0;
pub const NEIGHBORS: usize = 1;
pub const BATCH_SIZE: usize = 100_000;
pub const BATCH_SIZE_LOG: u64 = 100_000;

pub struct Route {
    pub source: Coord,
    pub destination: Coord,
    pub time: f32,
}

pub struct Coord {
    pub lat: f32,
    pub lon: f32,
}