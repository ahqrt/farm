use rkyv::Deserialize;
use std::path::{Path, PathBuf};

use farmfe_macro_cache_item::cache_item;

use crate::module::Module;
use crate::plugin::PluginAnalyzeDepsHookResultEntry;
use crate::{deserialize, serialize};

pub struct ModuleCacheManager {
  cache_dir: PathBuf,
}

#[cache_item]
pub struct CachedModule {
  pub module: Module,
  pub deps: Vec<PluginAnalyzeDepsHookResultEntry>,
}

impl ModuleCacheManager {
  pub fn new(root: &str) -> Self {
    Self {
      cache_dir: Path::new(root)
        .join("node_modules/")
        .join(".farm")
        .join("cache"),
    }
  }

  pub fn has_module_cache(&self, code_hash: &str) -> bool {
    let path = self.cache_dir.join(code_hash);
    path.exists()
  }

  pub fn set_module_cache(&self, code_hash: &str, module: &CachedModule) {
    let bytes = serialize!(module);
    let path = self.cache_dir.join(code_hash);
    std::fs::write(path, bytes).unwrap();
  }

  pub fn get_module_cache(&self, code_hash: &str) -> CachedModule {
    let path = self.cache_dir.join(code_hash);
    let bytes = std::fs::read(path).unwrap();
    deserialize!(&bytes, CachedModule)
  }
}
