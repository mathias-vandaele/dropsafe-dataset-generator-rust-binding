pub fn lat_lon_to_three_d(coord: [f32; 2]) -> [f32; 3] {
    let lat_rad = coord[0].to_radians();
    let lon_rad = coord[1].to_radians();
    [
        lat_rad.cos() * lon_rad.cos(),
        lat_rad.cos() * lon_rad.sin(),
        lat_rad.sin(),
    ]
}