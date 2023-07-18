pub mod module_cache;

/// All cache related operation are charged by [CacheManager]
pub struct CacheManager {
  module_cache: module_cache::ModuleCacheManager,
}

impl CacheManager {
  pub fn new(root: &str) -> Self {
    Self {
      module_cache: module_cache::ModuleCacheManager::new(root),
    }
  }
}
