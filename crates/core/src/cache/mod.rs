use crate::config::Mode;

pub mod module_cache;

/// All cache related operation are charged by [CacheManager]
pub struct CacheManager {
  pub module_cache: module_cache::ModuleCacheManager,
}

impl CacheManager {
  pub fn new(cache_dir: &str, namespace: &str, mode: Mode) -> Self {
    Self {
      module_cache: module_cache::ModuleCacheManager::new(cache_dir, namespace, mode),
    }
  }
}
