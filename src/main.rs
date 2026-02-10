mod config;
mod data_loader;
mod models;
mod projection;

use crate::models::{Coord, EARTH_RADIUS_KM, NEIGHBORS, Route};
use crate::projection::lat_lon_to_three_d;
use itertools::Itertools;
use kiddo::SquaredEuclidean;
use osrm_binding::algorithm::Algorithm;
use osrm_binding::osrm_engine::OsrmEngine;
use osrm_binding::point::Point;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::thread;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    let conf = config::get_config();

    let (values, kd_tree) = data_loader::load_coords_and_build_tree(&conf.input_address_file)?;
    let rings = data_loader::create_all_rings();
    let engine: OsrmEngine = OsrmEngine::new(&conf.osrm_file_ch, Algorithm::CH)
        .expect("Failed to initialize OSRM engine");

    let (tx, rx) = crossbeam::channel::unbounded::<String>();

    let handle = thread::spawn(move || {
        let mut count = 0u64;
        let mut previous_count = 0u64;
        let mut start_time = Instant::now();
        let file = File::create(&conf.output_file).unwrap();
        let mut writer = BufWriter::new(file);
        for line in rx {
            writer.write(line.as_bytes()).unwrap();
            count = count + 1;
            if count % 1000 == 0 {
                let request_treated = count - previous_count;
                let elapsed = start_time.elapsed().as_secs_f64();
                let rps = request_treated as f64 / elapsed;
                println!("Update: {} addresses processed - {:.2} RPS", count, rps);
                previous_count = count;
                start_time = Instant::now();
            }
        }
    });

    rings.iter().for_each(|ring| {
        println!("Treating ring: {}km", ring);
        values
            .par_iter()
            .for_each_with(tx.clone(), |tx, origin_point| {
                let around = points_on_circle(origin_point[0], origin_point[1], *ring);
                let durations = around
                    .into_iter()
                    .map(|point| {
                        kd_tree.approx_nearest_one::<SquaredEuclidean>(&lat_lon_to_three_d(point))
                    })
                    .filter_map(|nearest| {
                        engine
                            .simple_route(
                                Point {
                                    latitude: values[nearest.item as usize][0] as f64,
                                    longitude: values[nearest.item as usize][1] as f64,
                                },
                                Point {
                                    latitude: origin_point[0] as f64,
                                    longitude: origin_point[1] as f64,
                                },
                            )
                            .ok()
                            .map(|route| (values[nearest.item as usize], route.durations))
                    })
                    .collect::<Vec<([f32; 2], f64)>>();

                for (destination_point, duration) in durations {
                    let route = Route {
                        source: Coord {
                            lat: destination_point[0],
                            lon: destination_point[1],
                        },
                        destination: Coord {
                            lat: origin_point[0],
                            lon: origin_point[1],
                        },
                        time: duration as f32,
                    };
                    let str = format!(
                        "{}{}",
                        serde_json::to_string(&route).expect("Could not format String"),
                        "\n"
                    );
                    tx.send(str).unwrap();
                }
            })
    });

    drop(tx);
    handle.join().unwrap();

    Ok(())
}

fn points_on_circle(center_lat_deg: f32, center_lon_deg: f32, radius_km: f32) -> Vec<[f32; 2]> {
    let lat_rad = center_lat_deg.to_radians();
    let lon_rad = center_lon_deg.to_radians();
    let angular_distance = radius_km / EARTH_RADIUS_KM;

    let mut points = Vec::with_capacity(NEIGHBORS as usize);

    for i in 0..NEIGHBORS {
        let bearing = (i as f32) * (2.0 * PI / NEIGHBORS as f32);

        let point_lat_rad = (lat_rad.sin() * angular_distance.cos()
            + lat_rad.cos() * angular_distance.sin() * bearing.cos())
        .asin();

        let point_lon_rad = lon_rad
            + (bearing.sin() * angular_distance.sin() * lat_rad.cos())
                .atan2(angular_distance.cos() - lat_rad.sin() * point_lat_rad.sin());

        points.push([point_lat_rad.to_degrees(), point_lon_rad.to_degrees()]);
    }
    points
}
