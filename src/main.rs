use std::time::Duration;
use std::error::Error;
use std::sync::Arc;

use eframe::{egui, NativeOptions};
use egui::ViewportBuilder;
use glam::Mat4;
use log::{info, warn};
use memprocfs::Vmm;
use tokio::sync::watch::channel;

use crate::{offsets::global::GlobalOffsets, overlay::OverlayApp};
use memprocfs;

mod core;
mod entities;
mod globals;
mod offsets;
mod overlay;
mod types;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let dma_args = vec!["", "-device", "fpga"];
    let vmm: Arc<Vmm<'static>> = Arc::new(Vmm::new("vmm.dll", &dma_args)?);
    info!("Connected to DMA");

    const GAME_PROCESS_NAME: &str = "r5apex_dx12.exe";
    const PROCESS_WAIT_INTERVAL_MS: u64 = 200;

    let game_process = loop {
        match vmm.process_from_name(GAME_PROCESS_NAME) {
            Ok(proc) => {
                info!("Process found!");
                break proc;
            }
            Err(_) => {
                warn!("Waiting for {}...", GAME_PROCESS_NAME);
                std::thread::sleep(Duration::from_millis(PROCESS_WAIT_INTERVAL_MS));
            }
        }
    };

    info!("Fixing CR3...");
    utils::fix_cr3(&vmm, &game_process, GAME_PROCESS_NAME, game_process.pid)?;
    info!("CR3 fixed!");

    let module_base = game_process.get_module_base(GAME_PROCESS_NAME)?;
    crate::globals::set_module_base(module_base)?;
    info!("Module base: 0x{:X}", module_base);

    let entity_list = module_base + GlobalOffsets::ENTITYLIST;

    loop {
        let first_entity = game_process.mem_read_as::<u64>(entity_list, 0)?;
        if first_entity != 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
        info!("Waiting for entity list...");
    }

    // Overlay wiring
    const MAX_ENTRIES: usize = 128;
    let local_player =
        game_process.mem_read_as::<u64>(module_base + GlobalOffsets::LOCAL_ENT, 0)?;
    info!("Local player: {}", local_player);

    let (tx, rx) = channel::<crate::types::Snapshot>(crate::types::Snapshot {
        aabbs: Vec::new(),
        view: Mat4::IDENTITY,
        entities: Vec::new(),
    });

    // Unified sampler: single refresh loop replacing previous slow/fast producers
    {
        use crate::core::reader::MemprocfsGameReader;
        use crate::core::sampler::{SamplerConfig, spawn_sampler};

        let reader = MemprocfsGameReader::new(vmm.clone(), GAME_PROCESS_NAME, module_base);
        let cfg = SamplerConfig {
            base_tick_ms: 2,
            max_entries: MAX_ENTRIES,
            full_refresh_every_n: 20,
        };
        spawn_sampler(reader, local_player, cfg, tx.clone()).await;
    }

    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_decorations(true)
            .with_transparent(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "overlay",
        native_options,
        Box::new(move |_| Box::new(OverlayApp::new(rx, local_player))),
    )?;

    Ok(())
}
