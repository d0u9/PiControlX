use log;
use super::*;

use simplelog::{TestLogger, ConfigBuilder, LevelFilter};
use std::sync::Once;

static START: Once = Once::new();

fn test_init() {
    START.call_once(|| {
        TestLogger::init(LevelFilter::Info,
                         ConfigBuilder::new()
                             .set_location_level(LevelFilter::Error)
                             .build(),
                        ).unwrap();
    });
    println!("");
}

#[test]
fn test_hello() {
    test_init();
    log::warn!("Running...");
}

#[test]
fn test_scan() {
    let disks = get_disks();
    dbg!(disks);
}
