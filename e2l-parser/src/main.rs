mod e2gw_rpc_client;
mod e2gw_rpc_server;
mod e2l_active_directory;
mod e2l_crypto;
mod e2l_module;
mod e2l_mqtt_client;
mod json_structs;
mod lorawan_structs;
mod mqtt_client;

#[macro_use]
extern crate lazy_static;
extern crate base64;
extern crate core;
extern crate dotenv;
extern crate getopts;
extern crate lorawan_encoding;
extern crate p256;
extern crate rand;
extern crate rumqttc;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use e2l_module::e2l_module::e2l_module::E2LModule;

fn info(msg: String) {
    if true {
        //         println!("\nINFO: {}\n", msg);
        println!("INFO: {}", msg);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e2l_module: E2LModule = E2LModule::new().await;
    info(format!("E2L Module Initialised!"));
    return e2l_module.run().await;
}
