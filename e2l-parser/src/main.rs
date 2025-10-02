// #[macro_use]
// extern crate lazy_static;

use e2l_parser::E2LModule;


fn info(msg: String) {
    if true {
        //println!("\nINFO: {}\n", msg);
        println!("INFO: {}", msg);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let e2l_module: E2LModule = e2l_parser::E2LModule::new().await;
    info(format!("E2L Module Initialised!"));
    return e2l_module.run().await;
}
