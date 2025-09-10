#![warn(unused_extern_crates)]
mod e2l_active_directory;
mod e2l_crypto;
mod e2l_module;
mod e2l_mqtt_client;
mod json_structs;
mod lorawan_structs;
mod e2l_end_device;

// #[macro_use]
// extern crate lazy_static;

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
