use std::error::Error;
use std::thread;
use std::time;

use eframe::egui;
use glam::{Mat4, Vec3};
use memprocfs::{CONFIG_OPT_PROCESS_DTB, Vmm, VmmProcess};

pub fn world_to_screen(pos: Vec3, view: Mat4, width: f32, height: f32) -> Option<egui::Pos2> {
    // Convert Mat4 to array for matrix indexing (column-major order)
    let matrix = view.to_cols_array();

    // Calculate w component
    let w = matrix[12] * pos.x + matrix[13] * pos.y + matrix[14] * pos.z + matrix[15];

    if w < 0.01 {
        return None;
    }

    // Calculate x and y components
    let mut x = matrix[0] * pos.x + matrix[1] * pos.y + matrix[2] * pos.z + matrix[3];
    let mut y = matrix[4] * pos.x + matrix[5] * pos.y + matrix[6] * pos.z + matrix[7];

    // Apply perspective division
    let invw = 1.0 / w;
    x *= invw;
    y *= invw;

    // Convert to screen coordinates
    let screen_x = width / 2.0;
    let screen_y = height / 2.0;

    let final_x = screen_x + 0.5 * x * width + 0.5;
    let final_y = screen_y - 0.5 * y * height + 0.5;

    Some(egui::pos2(final_x, final_y))
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
