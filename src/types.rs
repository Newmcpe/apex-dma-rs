use glam::Mat4;
use glam::Vec3;

use crate::entities::Entity;

pub struct Snapshot {
    pub view: Mat4,
    pub entities: Vec<Entity>,
    pub aabbs: Vec<(Vec3, Vec3)>, // (mins, maxs) per entity in local space
}
