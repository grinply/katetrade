use std::{env, fs};
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf};
use serde::{Deserialize, Serialize};

pub const S3_BUCKET_HISTORICAL_DATA: &'static str = "profitpirates";
const CONFIGURATION_FOLDER: &'static str = ".kate";
const CONFIGURATION_FILE: &'static str = "config.json";
const DEFAULT_MARKET_CONNECTOR_URL: &'static str = "http://market-connector.grinply.com";
const DEFAULT_MARKET_CONNECTOR_SOCKET_URL: &'static str = "http://market-connector.grinply.com:8022";

#[derive(Clone, Serialize, Deserialize)]
pub struct KateConfiguration {
    pub market_connector_url: String,
    pub market_connector_socket_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_account_id: Option<String>,
}

pub fn load_configuration() -> KateConfiguration {
    let config_file_path = kate_home_directory()
        .join(format!("{}/{}", CONFIGURATION_FOLDER, CONFIGURATION_FILE).as_str());

    if !config_file_path.exists(){
        println!("config file doesn't exist, initializing configuration on {}", config_file_path.to_str().unwrap());
        fs::create_dir_all(kate_configuration_directory().join(CONFIGURATION_FOLDER))
            .expect("failed while trying to initialize configuration folder");

        let mut config_file = File::create(config_file_path.clone())
            .expect("failed to create configuration file");

        let config_json = serde_json::to_string_pretty(&KateConfiguration{
            market_connector_url: String::from(DEFAULT_MARKET_CONNECTOR_URL),
            market_connector_socket_url: String::from(DEFAULT_MARKET_CONNECTOR_SOCKET_URL),
            default_account_id: None,
        }).expect("failed to serialize kate configuration");

        write!(config_file, "{}", config_json)
            .expect(format!("failed write default configuration to {}", CONFIGURATION_FILE).as_str());
    }

    let json_config_data = fs::read_to_string(config_file_path)
        .expect("Unable to read config file");

    let configuration: KateConfiguration = serde_json::from_str(json_config_data.as_str())
        .expect("JSON configuration file is broken");

    configuration
}

/// default location where all the application configuration and files are stored
/// by default it uses the `home` directory defined by your system
/// your can customize the location by defining the environment variable KATE_HOME
#[cfg(not(test))]
pub fn kate_configuration_directory() -> PathBuf{
    kate_home_directory().join(CONFIGURATION_FOLDER)
}


/// home directory that will only be return only when running tests
#[cfg(test)]
pub fn kate_configuration_directory() -> PathBuf{
    PathBuf::from("resources")
}

/// uses either the default home folder or the value defined by the environment variable KATE_HOME
fn kate_home_directory() -> PathBuf {
    if let Ok(kate_directory) = env::var("KATE_HOME") {
        PathBuf::from(kate_directory)
    }else{
        PathBuf::from(env::var("HOME").expect("Could not get home directory"))
    }
}

