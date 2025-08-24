use std::error::Error;

use crate::projection::lat_lon_to_three_d;
use csv::ReaderBuilder;
use kiddo::ImmutableKdTree;
use rand::seq::SliceRandom;

pub fn create_all_rings() -> Vec<f32> {
    let mut rings: Vec<f32> = vec![1.0, 2.0, 3.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 150.0, 200.0, 250.0, 300.0];
    rings.reverse();
    rings
}

pub fn load_coords_and_build_tree(
    file_path: &str,
) -> Result<(Vec<[f32; 2]>, ImmutableKdTree<f32, 3>), Box<dyn Error>> {
    println!("{}", file_path);
    let file: std::fs::File = std::fs::File::open(file_path).expect("Addresses file was not found");

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_reader(file);

    let header_map: std::collections::HashMap<_, _> = rdr
        .headers()?
        .clone()
        .iter()
        .enumerate()
        .map(|(idx, header)| (header.to_string(), idx))
        .collect();

    let lat_index = header_map.get("lat").ok_or("Column 'lat' not found")?;
    let lon_index = header_map.get("lon").ok_or("Column 'lon' not found")?;

    println!("🌳  Building the k-dimensional tree...");
    let mut values: Vec<[f32; 2]> = rdr
        .records()
        .filter_map(|record| match record {
            Ok(r) => { 
                Some([r[*lat_index].parse::<f32>().ok()?, r[*lon_index].parse::<f32>().ok()?])
            }
            Err(_) => None,
        })
        .collect::<Vec<[f32; 2]>>();

    values.shuffle(&mut rand::rng());

    let three_d_space = values.iter().map(|lat_lon| lat_lon_to_three_d(*lat_lon)).collect::<Vec<[f32; 3]>>();

    let kd_tree: ImmutableKdTree<f32, 3> = ImmutableKdTree::new_from_slice(&three_d_space);
    Ok((values, kd_tree))
}
