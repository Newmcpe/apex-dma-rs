#![allow(dead_code)]

// Global offsets - grouped via inherent associated consts on unit structs
pub mod global {
    pub struct GlobalOffsets;
    impl GlobalOffsets {
        pub const ENTITYLIST: u64 = 0x6283eb8;
        pub const LOCAL_ENT: u64 = 0x26e0498;
        pub const NAME_LIST: u64 = 0x8cbe910;
        pub const THIRDPERSON: u64 = 0x01e3b190 + 0x6c;
        pub const TIMESCALE: u64 = 0x01841ee0;
        pub const OBSERVER_LIST: u64 = 0x02022b50 + 0x20C8;
    }
}

// Entity offsets - grouped via inherent associated consts on unit structs
pub mod entity {
    pub struct EntityOffsets;
    impl EntityOffsets {
        pub const TEAM: u64 = 0x334;
        pub const HEALTH: u64 = 0x324;
        pub const MAX_HEALTH: u64 = 0x468;
        pub const SHIELD: u64 = 0x1a0;
        pub const MAX_SHIELD: u64 = 0x1A4;
        pub const NAME: u64 = 0x0479;
        pub const NAME_INDEX: u64 = 0x38;
        pub const SIGN_NAME: u64 = 0x0478;
        pub const ABS_VELOCITY: u64 = 0x170;
        pub const VISIBLE_TIME: u64 = 0x1990;
        pub const ZOOMING: u64 = 0x1be1;
        pub const THIRDPERSON_SV: u64 = 0x36c8;
        pub const YAW: u64 = 0x223c - 0x8;
    }
}

// State offsets - grouped via inherent associated consts on unit structs
pub mod state {
    pub struct StateOffsets;
    impl StateOffsets {
        pub const LIFE_STATE: u64 = 0x0690;
        pub const BLEED_OUT_STATE: u64 = 0x2760;
    }
}

// Position and bone offsets - grouped via inherent associated consts on unit structs
pub mod position {
    pub struct PositionOffsets;
    impl PositionOffsets {
        pub const ORIGIN: u64 = 0x017c;
        pub const BONES: u64 = 0x0da0 + 0x48;
        pub const STUDIOHDR: u64 = 0xfd0;
        pub const COLLISION: u64 = 0x3b8;
        pub const COLLISION_MINS: u64 = 0x10;
        pub const COLLISION_MAXS: u64 = 0x1c;
    }
}

// Camera and view offsets - grouped via inherent associated consts on unit structs
pub mod camera {
    pub struct CameraOffsets;
    impl CameraOffsets {
        pub const AIMPUNCH: u64 = 0x2438;
        pub const CAMERAPOS: u64 = 0x1ee0;
        pub const VIEWANGLES: u64 = 0x2534 - 0x14;
        pub const BREATH_ANGLES: u64 = Self::VIEWANGLES - 0x10;
        pub const VIEW_RENDER: u64 = 0x3d3e018;
        pub const VIEW_MATRIX: u64 = 0x11a350; // 4x4 view/projection matrix
    }
}

// Observer offsets - grouped via inherent associated consts on unit structs
pub mod observer {
    pub struct ObserverOffsets;
    impl ObserverOffsets {
        pub const OBSERVER_MODE: u64 = 0x34a4;
        pub const OBSERVING_TARGET: u64 = 0x34b0;
    }
}
