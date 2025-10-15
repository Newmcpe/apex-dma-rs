use std::error::Error;
use std::thread;
use std::time;

use eframe::egui;
use egui::{pos2, Pos2};
use glam::{Mat4, Vec3};
use memprocfs::{CONFIG_OPT_PROCESS_DTB, Vmm, VmmProcess};

pub fn world_to_screen(pos: Vec3, view: Mat4, width: f32, height: f32) -> Option<Pos2> {
    let clip = view.transpose() * pos.extend(1.0);
    if clip.w.abs() < 0.001 {
        return None;
    }

    let ndc = clip.truncate() / clip.w;

    let final_x = (ndc.x * 0.5 + 0.5) * width;
    let final_y = (1.0 - (ndc.y * 0.5 + 0.5)) * height;

    Some(pos2(final_x, final_y))
}

pub fn fix_cr3(
    vmm: &Vmm,
    process: &VmmProcess,
    target_module: &str,
    pid: u32,
) -> Result<bool, Box<dyn Error>> {
    const PROGRESS_PATH: &str = "\\misc\\procinfo\\progress_percent.txt";
    const DTB_PATH: &str = "\\misc\\procinfo\\dtb.txt";

    while vmm
        .vfs_read(PROGRESS_PATH, 3, 0)
        .ok()
        .filter(|p| p.len() == 3)
        .is_none()
    {
        thread::sleep(time::Duration::from_millis(500));
    }

    let dtbs = vmm.vfs_read(DTB_PATH, 0x80000, 0)?;
    let config = CONFIG_OPT_PROCESS_DTB | pid as u64;

    String::from_utf8_lossy(&dtbs)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace().filter(|s| !s.is_empty());
            match (parts.next(), parts.next(), parts.next()) {
                (Some(_), Some("0"), Some(dtb)) => u64::from_str_radix(dtb, 16).ok(),
                _ => None,
            }
        })
        .find(|&dtb| {
            vmm.set_config(config, dtb).is_ok() && process.get_module_base(target_module).is_ok()
        })
        .map_or(Ok(false), |_| Ok(true))
}

// read_view_matrix moved to core::reader::MemprocfsGameReader
