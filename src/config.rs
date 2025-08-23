use std::{env, sync::OnceLock};

#[warn(dead_code)]
pub struct Config {
    pub output_file  : String,
    pub input_address_file : String,
    pub osrm_file_mld : String,
    pub osrm_file_ch : String,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[warn(dead_code)]
pub fn get_config() -> &'static Config {
    if let Err(err) = dotenvy::dotenv() {
        if !err.not_found() {
            eprintln!("⚠️ Erreur lors du chargement de .env : {err}");
        }else {
            println!(".env not found, continuing")
        }
    }

    CONFIG.get_or_init(|| {
        eprintln!("--- Config initialisation ---");
        Config {
            output_file: env::var("OUTPUT_FILE")
                .expect("OUTPUT_FILE env var is missing"),
            input_address_file: env::var("INPUT_ADDRESS_FILE")
                .expect("INPUT_ADDRESS_FILE env var is missing"),
            osrm_file_mld: env::var("OSRM_FILE_MLD")
                .expect("OSRM_FILE_MLD env var is missing"),
            osrm_file_ch: env::var("OSRM_FILE_CH")
                .expect("OSRM_FILE_CH env var is missing"),
        }
    })
}