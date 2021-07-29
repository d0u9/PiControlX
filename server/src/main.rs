use tokio::runtime::Runtime;

extern crate lib;

async fn tokio_main() {
    lib::lib_main().await;
}

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(tokio_main());
    println!("Hello, world!");
}
