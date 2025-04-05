use std::env;
use config;


pub fn get_value(settings: &config::Config, key: &str) -> String {

    let value = settings
        .get_str(key)
        .or_else(|_| env::var(key).map_err(|e| config::ConfigError::Message(e.to_string())))
        .map(|value| {
            if value.trim().is_empty() {
                panic!("Configuration key '{}' is set but empty.", key);
            }
            return value;
        })
        .expect(&format!("Failed to read '{}' from config or environment.", key));
    return value;
}


#[allow(non_snake_case)]
pub fn load_settings(name: &str) -> (String, String, String, String, u16) {

    let mut settings = config::Config::default();

    if settings.merge(config::File::with_name(name).required(false)).is_err() {
        eprintln!("Warning: Failed to open configuration file, falling back to environment variables.");
    }

    let RPC_URL = get_value(&settings, "RPC_URL");
    let RPC_USER = get_value(&settings, "RPC_USER");
    let RPC_PASSWORD = get_value(&settings, "RPC_PASSWORD");
    let SERVER_ADDR = get_value(&settings, "SERVER_ADDR");
    let SERVER_PORT: u16 = get_value(&settings, "SERVER_PORT").parse().expect("SERVER_PORT is not a valid port number.");

    return (RPC_URL, RPC_USER, RPC_PASSWORD, SERVER_ADDR, SERVER_PORT);
}

