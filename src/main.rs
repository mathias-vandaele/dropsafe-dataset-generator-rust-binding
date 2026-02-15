mod config;
mod data_loader;
mod models;
mod projection;

use crate::models::{BATCH_SIZE, BATCH_SIZE_LOG, Coord, EARTH_RADIUS_KM, NEIGHBORS, Route};
use crate::projection::lat_lon_to_three_d;
use arrow2::array::Float32Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::{Field, Schema};
use arrow2::io::parquet::write::{
    CompressionOptions, Encoding, FileWriter, RowGroupIterator, Version, WriteOptions, transverse,
};
use kiddo::SquaredEuclidean;
use osrm_binding::algorithm::Algorithm;
use osrm_binding::osrm_engine::OsrmEngine;
use osrm_binding::point::Point;
use rand::Rng;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::thread;
use std::time::Instant;

enum RouteResult {
    Calculable(Route),
    NotCalculable,
}

fn main() -> Result<(), Box<dyn Error>> {
    let conf = config::get_config();

    let (values, kd_tree) = data_loader::load_coords_and_build_tree(&conf.input_address_file)?;
    let rings = data_loader::create_all_rings();
    let engine: OsrmEngine = OsrmEngine::new(&conf.osrm_file_ch, Algorithm::CH)
        .expect("Failed to initialize OSRM engine");

    let (tx, rx) = crossbeam::channel::unbounded::<RouteResult>();
    let total_expected = values.len() * rings.len() * NEIGHBORS as usize;

    let handle = thread::spawn(move || {
        let mut success_count = 0u64;
        let mut error_count = 0u64;
        let mut previous_total = 0u64;
        let mut start_time = Instant::now();

        let schema = Schema::from(vec![
            Field::new("source_lat", arrow2::datatypes::DataType::Float32, false),
            Field::new("source_lon", arrow2::datatypes::DataType::Float32, false),
            Field::new("dest_lat", arrow2::datatypes::DataType::Float32, false),
            Field::new("dest_lon", arrow2::datatypes::DataType::Float32, false),
            Field::new("time", arrow2::datatypes::DataType::Float32, false),
        ]);

        let file = File::create(&conf.output_file).expect("Failed to create output file");
        let options = WriteOptions {
            write_statistics: true,
            compression: CompressionOptions::Snappy,
            version: Version::V2,
            data_pagesize_limit: None,
        };
        let mut writer = FileWriter::try_new(file, schema.clone(), options)
            .expect("Failed to create Parquet writer");

        // Buffers pour stocker temporairement les données
        let mut buf_source_lat = Vec::with_capacity(BATCH_SIZE);
        let mut buf_source_lon = Vec::with_capacity(BATCH_SIZE);
        let mut buf_dest_lat = Vec::with_capacity(BATCH_SIZE);
        let mut buf_dest_lon = Vec::with_capacity(BATCH_SIZE);
        let mut buf_time = Vec::with_capacity(BATCH_SIZE);

        for result in rx {
            match result {
                RouteResult::Calculable(route) => {
                    buf_source_lat.push(route.source.lat);
                    buf_source_lon.push(route.source.lon);
                    buf_dest_lat.push(route.destination.lat);
                    buf_dest_lon.push(route.destination.lon);
                    buf_time.push(route.time);
                    success_count += 1;
                }
                RouteResult::NotCalculable => error_count += 1,
            }

            if buf_source_lat.len() >= BATCH_SIZE {
                persist_batch(
                    &schema,
                    options,
                    &mut writer,
                    &mut buf_source_lat,
                    &mut buf_source_lon,
                    &mut buf_dest_lat,
                    &mut buf_dest_lon,
                    &mut buf_time,
                );
            }

            let total_count = success_count + error_count;
            if total_count % BATCH_SIZE_LOG == 0 {
                let request_treated = total_count - previous_total;
                let elapsed = start_time.elapsed().as_secs_f64();
                let rps = request_treated as f64 / elapsed;
                let progress = (total_count as f64 / total_expected as f64) * 100.0;
                println!(
                    "Progress: {:.2}% ({}/{}) - Success: {} - Errors: {} - {:.2} RPS",
                    progress, total_count, total_expected, success_count, error_count, rps
                );
                previous_total = total_count;
                start_time = Instant::now();
            }
        }
        // Write any remaining data in buffers
        persist_batch(
            &schema,
            options,
            &mut writer,
            &mut buf_source_lat,
            &mut buf_source_lon,
            &mut buf_dest_lat,
            &mut buf_dest_lon,
            &mut buf_time,
        );
        writer.end(None).expect("Failed to close writer");
        println!(
            "Final: Success: {} - Errors: {} - Total: {}",
            success_count,
            error_count,
            success_count + error_count
        );
    });

    rings.par_iter().for_each_with(tx.clone(), |tx, ring| {
        values.iter().for_each(|origin_point| {
            points_on_circle(origin_point[0], origin_point[1], *ring)
                .into_iter()
                .map(|point| {
                    kd_tree.approx_nearest_one::<SquaredEuclidean>(&lat_lon_to_three_d(point))
                })
                .map(|nearest| {
                    match engine.simple_route(
                        Point {
                            latitude: values[nearest.item as usize][0] as f64,
                            longitude: values[nearest.item as usize][1] as f64,
                        },
                        Point {
                            latitude: origin_point[0] as f64,
                            longitude: origin_point[1] as f64,
                        },
                    ) {
                        Ok(route) => RouteResult::Calculable(Route {
                            source: Coord {
                                lat: values[nearest.item as usize][0],
                                lon: values[nearest.item as usize][1],
                            },
                            destination: Coord {
                                lat: origin_point[0],
                                lon: origin_point[1],
                            },
                            time: route.durations as f32,
                        }),
                        Err(_) => RouteResult::NotCalculable,
                    }
                })
                .for_each(|result| {
                    tx.send(result)
                        .map_err(move |e| println!("Failed to send result: {}", e))
                        .unwrap();
                });
        })
    });

    drop(tx);
    handle.join().unwrap();

    Ok(())
}

fn persist_batch(
    schema: &Schema,
    options: WriteOptions,
    mut writer: &mut FileWriter<File>,
    mut buf_source_lat: &mut Vec<f32>,
    mut buf_source_lon: &mut Vec<f32>,
    mut buf_dest_lat: &mut Vec<f32>,
    mut buf_dest_lon: &mut Vec<f32>,
    mut buf_time: &mut Vec<f32>,
) {
    let arrays = vec![
        Float32Array::from_vec(std::mem::take(&mut buf_source_lat)).boxed(),
        Float32Array::from_vec(std::mem::take(&mut buf_source_lon)).boxed(),
        Float32Array::from_vec(std::mem::take(&mut buf_dest_lat)).boxed(),
        Float32Array::from_vec(std::mem::take(&mut buf_dest_lon)).boxed(),
        Float32Array::from_vec(std::mem::take(&mut buf_time)).boxed(),
    ];
    let chunk = Chunk::new(arrays);

    let encodings = schema
        .fields
        .iter()
        .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
        .collect();

    let row_groups =
        RowGroupIterator::try_new(vec![Ok(chunk)].into_iter(), &schema, options, encodings)
            .expect("Failed to create row group iterator");

    for group in row_groups {
        writer
            .write(group.expect("Failed to create row group"))
            .expect("Failed to write row group");
    }
}

fn points_on_circle(center_lat_deg: f32, center_lon_deg: f32, radius_km: f32) -> Vec<[f32; 2]> {
    let lat_rad = center_lat_deg.to_radians();
    let lon_rad = center_lon_deg.to_radians();
    let angular_distance = radius_km / EARTH_RADIUS_KM;

    let mut points = Vec::with_capacity(NEIGHBORS as usize);

    let random_offset: f32 = rand::rng().random::<f32>() * 2.0 * PI;

    for i in 0..NEIGHBORS {
        let bearing = random_offset + ((i as f32) * (2.0 * PI / NEIGHBORS as f32));

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
