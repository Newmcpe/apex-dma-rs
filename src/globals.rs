use std::sync::OnceLock;

static MODULE_BASE: OnceLock<u64> = OnceLock::new();

pub fn set_module_base(module_base: u64) -> Result<(), &'static str> {
    MODULE_BASE
        .set(module_base)
        .map_err(|_| "Module base already set")
}

pub fn get_module_base() -> Result<u64, &'static str> {
    MODULE_BASE
        .get()
        .copied()
        .ok_or("Module base not initialized")
}
