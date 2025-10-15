use glam::Vec3;
use log::warn;
use memprocfs::{ResultEx, VmmProcess};

use crate::globals;
use crate::offsets::{entity::EntityOffsets, global::GlobalOffsets, position::PositionOffsets};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Entity {
    pub base: u64,
    pub team: u32,
    pub health: u32,
    pub shield: u32,
    pub name_index: i32,
    pub origin: Vec3,
    pub name: String,
    pub class_name: String,
}

impl Entity {}

pub fn read_entity_aabbs(process: &VmmProcess, bases: &[u64]) -> ResultEx<Vec<(Vec3, Vec3)>> {
    if bases.is_empty() {
        return Ok(Vec::new());
    }
    let mem_scatter =
        process.mem_scatter(memprocfs::FLAG_NOCACHE | memprocfs::FLAG_ZEROPAD_ON_FAIL)?;
    for &base in bases {
        let coll = base + PositionOffsets::COLLISION;
        let _ = mem_scatter.prepare_as::<[f32; 3]>(base + PositionOffsets::ORIGIN);
        let _ = mem_scatter.prepare_as::<[f32; 3]>(coll + PositionOffsets::COLLISION_MINS);
        let _ = mem_scatter.prepare_as::<[f32; 3]>(coll + PositionOffsets::COLLISION_MAXS);
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (aabb origin/mins/maxs): {}", e);
        return Err(e);
    }
    let mut out = Vec::with_capacity(bases.len());
    for &base in bases {
        let coll = base + PositionOffsets::COLLISION;
        let origin_arr = mem_scatter
            .read_as::<[f32; 3]>(base + PositionOffsets::ORIGIN)
            .unwrap_or([0.0; 3]);
        let origin = Vec3::from_array(origin_arr);
        let mins = mem_scatter
            .read_as::<[f32; 3]>(coll + PositionOffsets::COLLISION_MINS)
            .unwrap_or([0.0; 3]);
        let maxs = mem_scatter
            .read_as::<[f32; 3]>(coll + PositionOffsets::COLLISION_MAXS)
            .unwrap_or([0.0; 3]);
        let mins_vec = Vec3::from_array(mins) + origin;
        let maxs_vec = Vec3::from_array(maxs) + origin;
        out.push((mins_vec, maxs_vec));
    }
    Ok(out)
}

pub fn gather_entity_bases(
    process: &VmmProcess,
    entity_list: u64,
    limit: usize,
    _local_player: u64,
) -> ResultEx<Vec<u64>> {
    let mem_scatter =
        process.mem_scatter(memprocfs::FLAG_NOCACHE | memprocfs::FLAG_ZEROPAD_ON_FAIL)?;
    // Entity pointers are at entity_list + ((i + 1) << 5)
    for i in 0..limit {
        let addr = entity_list + (((i as u64) + 1) << 5);
        let _ = mem_scatter.prepare_as::<u64>(addr);
    }
    if let Err(e) = mem_scatter.execute() {
        warn!(
            "scatter execute failed in gather_entity_bases (entity list entries): {}",
            e
        );
        return Err(e);
    }
    let mut bases = Vec::with_capacity(limit);
    for i in 0..limit {
        let addr = entity_list + (((i as u64) + 1) << 5);
        let ptr = match mem_scatter.read_as::<u64>(addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("entity ptr read failed {:x}: {}", addr, e);
                0
            }
        };
        if ptr != 0 {
            bases.push(ptr);
        }
    }
    Ok(bases)
}

pub fn read_entities(process: &VmmProcess, bases: &[u64]) -> ResultEx<Vec<Entity>> {
    if bases.is_empty() {
        return Ok(Vec::new());
    }
    let mem_scatter =
        process.mem_scatter(memprocfs::FLAG_NOCACHE | memprocfs::FLAG_ZEROPAD_ON_FAIL)?;

    const NETWORKABLE_VTBL_OFFSET: u64 = 8 * 3; // entity_ptr + 8 * 3
    for &base in bases {
        let origin_addr = base + PositionOffsets::ORIGIN;
        let team_addr = base + EntityOffsets::TEAM;
        let health_addr = base + EntityOffsets::HEALTH;
        let shield_addr = base + EntityOffsets::SHIELD;
        let name_index_addr = base + EntityOffsets::NAME_INDEX;
        let vtable_ptr_addr = base + NETWORKABLE_VTBL_OFFSET;
        let _ = mem_scatter.prepare_as::<[f32; 3]>(origin_addr);
        let _ = mem_scatter.prepare_as::<u32>(team_addr);
        let _ = mem_scatter.prepare_as::<u32>(health_addr);
        let _ = mem_scatter.prepare_as::<u32>(shield_addr);
        let _ = mem_scatter.prepare_as::<i32>(name_index_addr);
        let _ = mem_scatter.prepare_as::<u64>(vtable_ptr_addr);
    }

    if let Err(e) = mem_scatter.execute() {
        warn!(
            "scatter execute failed at Pass A (base fields + vtable ptr): {}",
            e
        );
        return Err(e);
    }

    let mut origins = Vec::with_capacity(bases.len());
    let mut teams = Vec::with_capacity(bases.len());
    let mut healths = Vec::with_capacity(bases.len());
    let mut shields = Vec::with_capacity(bases.len());
    let mut name_indexes = Vec::with_capacity(bases.len());
    let mut vtptrs = Vec::with_capacity(bases.len());
    for &base in bases {
        let origin_addr = base + PositionOffsets::ORIGIN;
        let team_addr = base + EntityOffsets::TEAM;
        let health_addr = base + EntityOffsets::HEALTH;
        let shield_addr = base + EntityOffsets::SHIELD;
        let name_index_addr = base + EntityOffsets::NAME_INDEX;
        let vtable_ptr_addr = base + NETWORKABLE_VTBL_OFFSET;

        let origin = match mem_scatter.read_as::<[f32; 3]>(origin_addr) {
            Ok(arr) => Vec3::new(arr[0], arr[1], arr[2]),
            Err(e) => {
                warn!("origin read failed {:x}: {}", origin_addr, e);
                Vec3::ZERO
            }
        };
        let team = match mem_scatter.read_as::<u32>(team_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("team read failed {:x}: {}", team_addr, e);
                0
            }
        };
        let health = match mem_scatter.read_as::<u32>(health_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("health read failed {:x}: {}", health_addr, e);
                0
            }
        };
        let shield = match mem_scatter.read_as::<u32>(shield_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("shield read failed {:x}: {}", shield_addr, e);
                0
            }
        };
        let name_index = match mem_scatter.read_as::<i32>(name_index_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("name_index read failed {:x}: {}", name_index_addr, e);
                -1
            }
        };

        origins.push(origin);
        teams.push(team);
        healths.push(health);
        shields.push(shield);
        name_indexes.push(name_index);
        let vt = match mem_scatter.read_as::<u64>(vtable_ptr_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("vtable ptr read failed {:x}: {}", vtable_ptr_addr, e);
                0
            }
        };
        vtptrs.push(vt);
    }

    // Pass B: vtable -> get_client_class
    const GET_CLIENT_CLASS_INDEX: u64 = 8 * 3; // vfunc 3
    mem_scatter.clear()?;
    for &vt in &vtptrs {
        if vt != 0 {
            let _ = mem_scatter.prepare_as::<u64>(vt + GET_CLIENT_CLASS_INDEX);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (get_client_class vfunc ptr): {}", e);
        return Err(e);
    }
    let mut gcc_ptrs = Vec::with_capacity(bases.len());
    for &vt in &vtptrs {
        if vt == 0 {
            gcc_ptrs.push(0);
            continue;
        }
        let addr = vt + GET_CLIENT_CLASS_INDEX;
        let gcc = match mem_scatter.read_as::<u64>(addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("get_client_class ptr read failed {:x}: {}", addr, e);
                0
            }
        };
        gcc_ptrs.push(gcc);
    }

    // Pass C: rel32 disp at gcc+3 -> client_class_ptr
    const REL32_DISP_OFFSET: u64 = 3;
    mem_scatter.clear()?;
    for &gcc in &gcc_ptrs {
        if gcc != 0 {
            let _ = mem_scatter.prepare_as::<i32>(gcc + REL32_DISP_OFFSET);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (rel32 displacement read): {}", e);
        return Err(e);
    }
    let mut client_class_ptrs = Vec::with_capacity(bases.len());
    for &gcc in &gcc_ptrs {
        if gcc == 0 {
            client_class_ptrs.push(0);
            continue;
        }
        let disp_addr = gcc + REL32_DISP_OFFSET;
        let disp = match mem_scatter.read_as::<i32>(disp_addr) {
            Ok(v) => v as i64,
            Err(e) => {
                warn!("rel32 disp read failed {:x}: {}", disp_addr, e);
                0
            }
        };
        let ccp = if disp != 0 {
            ((gcc as i64).wrapping_add(disp).wrapping_add(7)) as u64
        } else {
            0
        };
        client_class_ptrs.push(ccp);
    }

    // Pass D: ClientClass.pNetworkName at +0x10
    const CLIENT_CLASS_NAME_PTR_OFFSET: u64 = 0x10;
    mem_scatter.clear()?;
    for &ccp in &client_class_ptrs {
        if ccp != 0 {
            let _ = mem_scatter.prepare_as::<u64>(ccp + CLIENT_CLASS_NAME_PTR_OFFSET);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (ClientClass name pointer): {}", e);
        return Err(e);
    }
    let mut class_name_ptrs = Vec::with_capacity(bases.len());
    for &ccp in &client_class_ptrs {
        if ccp == 0 {
            class_name_ptrs.push(0);
            continue;
        }
        let np_addr = ccp + CLIENT_CLASS_NAME_PTR_OFFSET;
        let np = match mem_scatter.read_as::<u64>(np_addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("name ptr read failed {:x}: {}", np_addr, e);
                0
            }
        };
        class_name_ptrs.push(np);
    }

    // Pass E: read 32-byte class names
    mem_scatter.clear()?;
    for &np in &class_name_ptrs {
        if np != 0 {
            let _ = mem_scatter.prepare_as::<[u8; 32]>(np);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (class name bytes): {}", e);
        return Err(e);
    }
    let mut class_names = Vec::with_capacity(bases.len());
    for &np in &class_name_ptrs {
        if np == 0 {
            class_names.push(String::new());
            continue;
        }
        let arr = match mem_scatter.read_as::<[u8; 32]>(np) {
            Ok(v) => v,
            Err(e) => {
                warn!("class name read failed {:x}: {}", np, e);
                [0u8; 32]
            }
        };
        let bytes = &arr[..];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        class_names.push(String::from_utf8_lossy(&bytes[..end]).to_string());
    }
    // Player name gathering via NAME_LIST
    const NAME_MAX: usize = 32;
    const NAME_ENTRY_SIZE: u64 = 0x18; // 24 bytes per entry
    let module_base = globals::get_module_base().unwrap_or(0);
    mem_scatter.clear()?;
    for &idx in &name_indexes {
        if idx > 0 && module_base != 0 {
            let index = (idx as u32 as u64).saturating_sub(1);
            let addr = module_base + GlobalOffsets::NAME_LIST + index * NAME_ENTRY_SIZE;
            let _ = mem_scatter.prepare_as::<u64>(addr);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (name list pointer): {}", e);
        return Err(e);
    }
    let mut name_value_ptrs = Vec::with_capacity(bases.len());
    for &idx in &name_indexes {
        if idx <= 0 || module_base == 0 {
            name_value_ptrs.push(0);
            continue;
        }
        let index = (idx as u32 as u64).saturating_sub(1);
        let addr = module_base + GlobalOffsets::NAME_LIST + index * NAME_ENTRY_SIZE;
        let ptr = match mem_scatter.read_as::<u64>(addr) {
            Ok(v) => v,
            Err(e) => {
                warn!("name list ptr read failed {:x}: {}", addr, e);
                0
            }
        };
        name_value_ptrs.push(ptr);
    }

    // Read actual player name bytes (32 bytes) from name_value_ptrs
    mem_scatter.clear()?;
    for &np in &name_value_ptrs {
        if np != 0 {
            let _ = mem_scatter.prepare_as::<[u8; NAME_MAX]>(np);
        }
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (player name bytes): {}", e);
        return Err(e);
    }
    let mut player_names = Vec::with_capacity(bases.len());
    for &np in &name_value_ptrs {
        if np == 0 {
            player_names.push(String::new());
            continue;
        }
        let arr = match mem_scatter.read_as::<[u8; NAME_MAX]>(np) {
            Ok(v) => v,
            Err(e) => {
                warn!("player name read failed {:x}: {}", np, e);
                [0u8; NAME_MAX]
            }
        };
        let bytes = &arr[..];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        player_names.push(String::from_utf8_lossy(&bytes[..end]).to_string());
    }

    let mut result = Vec::with_capacity(bases.len());
    for i in 0..bases.len() {
        result.push(Entity {
            base: bases[i],
            team: teams[i],
            health: healths[i],
            shield: shields[i],
            name_index: name_indexes[i],
            origin: origins[i],
            name: player_names.get(i).cloned().unwrap_or_default(),
            class_name: class_names.get(i).cloned().unwrap_or_default(),
        });
    }

    Ok(result)
}

pub fn read_entity_origins(process: &VmmProcess, bases: &[u64]) -> ResultEx<Vec<Vec3>> {
    if bases.is_empty() {
        return Ok(Vec::new());
    }
    let mem_scatter =
        process.mem_scatter(memprocfs::FLAG_NOCACHE | memprocfs::FLAG_ZEROPAD_ON_FAIL)?;
    for &base in bases {
        let origin_addr = base + PositionOffsets::ORIGIN;
        let _ = mem_scatter.prepare_as::<[f32; 3]>(origin_addr);
    }
    if let Err(e) = mem_scatter.execute() {
        warn!("scatter execute failed (origins fast path): {}", e);
        return Err(e);
    }
    let mut origins = Vec::with_capacity(bases.len());
    for &base in bases {
        let origin_addr = base + PositionOffsets::ORIGIN;
        let origin = match mem_scatter.read_as::<[f32; 3]>(origin_addr) {
            Ok(arr) => Vec3::new(arr[0], arr[1], arr[2]),
            Err(e) => {
                warn!("origin read failed {:x}: {}", origin_addr, e);
                Vec3::ZERO
            }
        };
        origins.push(origin);
    }
    Ok(origins)
}
