use std::sync::Arc;

use glam::{Mat4, Vec3};
use memprocfs::{ResultEx, Vmm, VmmProcess};

use crate::entities::{self, Entity};
use crate::offsets::camera::CameraOffsets;
use crate::offsets::global::GlobalOffsets;

pub trait GameReader {
    fn read_view_matrix(&self) -> Option<Mat4>;
    fn gather_entity_bases(&self, limit: usize, local_player: u64) -> ResultEx<Vec<u64>>;
    fn read_entities(&self, bases: &[u64]) -> ResultEx<Vec<Entity>>;
    fn read_entity_origins(&self, bases: &[u64]) -> ResultEx<Vec<Vec3>>;
    fn read_entity_aabbs(&self, bases: &[u64]) -> ResultEx<Vec<(Vec3, Vec3)>>;
}

pub struct MemprocfsGameReader {
    pub vmm: Arc<Vmm<'static>>,
    pub process_name: &'static str,
    pub module_base: u64,
    pub entity_list: u64,
}

impl MemprocfsGameReader {
    pub fn new(vmm: Arc<Vmm<'static>>, process_name: &'static str, module_base: u64) -> Self {
        let entity_list = module_base + GlobalOffsets::ENTITYLIST;
        Self {
            vmm,
            process_name,
            module_base,
            entity_list,
        }
    }

    fn with_process<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&VmmProcess) -> T,
    {
        match self.vmm.process_from_name(self.process_name) {
            Ok(proc) => Some(f(&proc)),
            Err(_) => None,
        }
    }
}

impl GameReader for MemprocfsGameReader {
    fn read_view_matrix(&self) -> Option<Mat4> {
        self.with_process(|process| {
            let view_renderer_ptr = process
                .mem_read_as::<u64>(
                    self.module_base + CameraOffsets::VIEW_RENDER,
                    memprocfs::FLAG_NOCACHE,
                )
                .ok()?;
            if view_renderer_ptr == 0 {
                return None;
            }
            let view_matrix_ptr = process
                .mem_read_as::<u64>(
                    view_renderer_ptr + CameraOffsets::VIEW_MATRIX,
                    memprocfs::FLAG_NOCACHE,
                )
                .ok()?;
            if view_matrix_ptr == 0 {
                return None;
            }
            let raw = process
                .mem_read_as::<[f32; 16]>(view_matrix_ptr, memprocfs::FLAG_NOCACHE)
                .ok()?;
            Some(Mat4::from_cols_array(&raw))
        })?
    }

    fn gather_entity_bases(&self, limit: usize, local_player: u64) -> ResultEx<Vec<u64>> {
        // reacquire process each call
        match self.vmm.process_from_name(self.process_name) {
            Ok(proc) => entities::gather_entity_bases(&proc, self.entity_list, limit, local_player),
            Err(e) => Err(e),
        }
    }

    fn read_entities(&self, bases: &[u64]) -> ResultEx<Vec<Entity>> {
        match self.vmm.process_from_name(self.process_name) {
            Ok(proc) => entities::read_entities(&proc, bases),
            Err(e) => Err(e),
        }
    }

    fn read_entity_origins(&self, bases: &[u64]) -> ResultEx<Vec<Vec3>> {
        match self.vmm.process_from_name(self.process_name) {
            Ok(proc) => entities::read_entity_origins(&proc, bases),
            Err(e) => Err(e),
        }
    }

    fn read_entity_aabbs(&self, bases: &[u64]) -> ResultEx<Vec<(Vec3, Vec3)>> {
        match self.vmm.process_from_name(self.process_name) {
            Ok(proc) => entities::read_entity_aabbs(&proc, bases),
            Err(e) => Err(e),
        }
    }
}
