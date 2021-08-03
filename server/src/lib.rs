use log;
use simplelog::{ColorChoice, LevelFilter, TermLogger, TerminalMode};
use tokio::signal;

mod caches;
mod public;
mod server;
use server::server::Server;

fn setup_logger() {
    TermLogger::init(
        LevelFilter::Debug,
        // simplelog::Config::default(),
        simplelog::ConfigBuilder::new()
            .set_location_level(LevelFilter::Debug)
            .set_target_level(LevelFilter::Debug)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

pub async fn lib_main() {
    setup_logger();

    log::warn!("async main start...");
    let addr = "[::1]:50051".parse().unwrap();
    let (mut server, server_handler) = Server::new(addr);
    let mut cache_manager = caches::CacheManager::new();
    cache_manager.create_caches(&mut server);

    let handler1 = tokio::spawn(async move {
        server.serve().await;
    });

    let cache_manager_handler = cache_manager.run();

    signal::ctrl_c().await.unwrap();
    tokio::join!(cache_manager_handler.shutdown(), server_handler.shutdown());
    handler1.await.unwrap();
}
