use log;
use simplelog::{ColorChoice, LevelFilter, TermLogger, TerminalMode};
use tokio::signal;

mod caches;
mod public;
mod server;
use server::server::Server;

fn setup_logger() {
    TermLogger::init(
        LevelFilter::Trace,
        simplelog::Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

pub async fn lib_main() {
    setup_logger();

    log::info!("async main start...");
    let (mut server, server_handler) = Server::new();
    let (mut cache_manager, cache_manager_handler) = caches::CacheManager::new();
    cache_manager.create_caches(&mut server);

    tokio::spawn(async move {
        server.serve().await;
    });

    tokio::spawn(async move {
        cache_manager.run();
    });

    signal::ctrl_c().await.unwrap();
    tokio::join!(server_handler.shutdown(), cache_manager_handler.shutdown());
}
