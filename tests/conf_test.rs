
use config::Config;
use std::env;

#[path = "../src/conf.rs"]
mod conf;

use crate::conf::load_settings;
use crate::conf::get_value;


fn get_environment_keys() -> &'static [&'static str] {
    return &["RPC_URL", "RPC_USER", "RPC_PASSWORD", "SERVER_ADDR", "SERVER_PORT"];
}


fn setup_enviroment() {
    for key in get_environment_keys() {
        env::set_var(key, "1234");
    }
}


fn clear_enviroment() {
    for key in get_environment_keys() {
        std::env::remove_var(key);
    }
}


#[test]
#[should_panic(expected = "Failed to read 'RPC_URL' from config or environment.: environment variable not found")]
fn test_missing_setting_in_config_file() {
    setup_enviroment();

    let settings = config::Config::default();

    env::remove_var("RPC_URL");

    let _ = get_value(&settings, "RPC_URL");
}


#[test]
fn test_get_value_from_config() {
    clear_enviroment();

    let mut settings = Config::default();
    settings.set("RPC_URL", "http://localhost:8545").unwrap();

    assert_eq!(get_value(&settings, "RPC_URL"), "http://localhost:8545");
}


#[test]
#[should_panic(expected = "Configuration key 'RPC_URL' is set but empty.")]
fn test_get_value_empty_in_config() {
    clear_enviroment();

    let mut settings = Config::default();
    settings.set("RPC_URL", "").unwrap();

    get_value(&settings, "RPC_URL");
}


#[test]
fn test_get_value_from_env() {
    setup_enviroment();

    env::set_var("RPC_URL", "http://env-url:8545");

    let (rpc_url, _, _, _, _) = load_settings("_Conf");
    assert_eq!(rpc_url, "http://env-url:8545");
}


#[test]
#[should_panic(expected = "Failed to read 'RPC_URL' from config or environment.")]
fn test_get_value_missing_in_both() {
    setup_enviroment();

    env::remove_var("RPC_URL");

    let _ = load_settings("_Conf");
}


#[test]
fn test_get_port_from_env() {

    setup_enviroment();

    env::set_var("SERVER_PORT", "9090");


    let (_, _, _, _, server_port) = load_settings("_Conf");
    assert_eq!(server_port, 9090);
}


#[test]
#[should_panic(expected = "SERVER_PORT is not a valid port number.: ParseIntError { kind: InvalidDigit }")]
fn test_get_port_invalid_value() {

    setup_enviroment();

    env::set_var("SERVER_PORT", "invalid");


    let _ = load_settings("_Conf");
}

