use std::{any::Any, sync::Arc};

use dashmap::DashMap;
use hashbrown::HashMap;
use parking_lot::{Mutex, RwLock};
use swc_common::{FilePathMapping, Globals, SourceMap};

use crate::{
  cache::CacheManager,
  config::Config,
  error::Result,
  module::{module_graph::ModuleGraph, module_group::ModuleGroupGraph, watch_graph::WatchGraph},
  plugin::{plugin_driver::PluginDriver, Plugin},
  resource::{resource_pot_map::ResourcePotMap, Resource},
  record::RecordManager,
};

/// Shared context through the whole compilation.
pub struct CompilationContext {
  pub config: Box<Config>,
  pub watch_graph: Box<RwLock<WatchGraph>>,
  pub module_graph: Box<RwLock<ModuleGraph>>,
  pub module_group_graph: Box<RwLock<ModuleGroupGraph>>,
  pub plugin_driver: Box<PluginDriver>,
  pub resource_pot_map: Box<RwLock<ResourcePotMap>>,
  pub resources_map: Box<Mutex<HashMap<String, Resource>>>,
  pub cache_manager: Box<CacheManager>,
  pub meta: Box<ContextMetaData>,
  pub record_manager: Box<RecordManager>
}

impl CompilationContext {
  pub fn new(config: Config, plugins: Vec<Arc<dyn Plugin>>) -> Result<Self> {
    let cache_config = config.persistent_cache.as_ref();

    let (cache_dir, namespace) = if cache_config.enabled() {
      let cache_config_obj = cache_config.as_obj(&config.root);
      (cache_config_obj.cache_dir, cache_config_obj.namespace)
    } else {
      ("".to_string(), "".to_string())
    };

    Ok(Self {
      watch_graph: Box::new(RwLock::new(WatchGraph::new())),
      module_graph: Box::new(RwLock::new(ModuleGraph::new())),
      module_group_graph: Box::new(RwLock::new(ModuleGroupGraph::new())),
      resource_pot_map: Box::new(RwLock::new(ResourcePotMap::new())),
      resources_map: Box::new(Mutex::new(HashMap::new())),
      cache_manager: Box::new(CacheManager::new(
        &cache_dir,
        &namespace,
        config.mode.clone(),
      )),
      config: Box::new(config),
      plugin_driver: Box::new(PluginDriver::new(plugins)),
      meta: Box::new(ContextMetaData::new()),
      record_manager: Box::new(RecordManager::new()),
    })
  }

  pub fn add_watch_files(&self, from: String, deps: Vec<&String>) -> Result<()> {
    // @import 'variable.scss'
    // @import './variable.scss'
    let mut watch_graph = self.watch_graph.write();

    for dep in deps {
      watch_graph.add_node(from.clone());

      watch_graph.add_node(dep.clone());

      watch_graph.add_edge(&from, dep)?;
    }

    Ok(())
  }
}

impl Default for CompilationContext {
  fn default() -> Self {
    Self::new(Config::default(), vec![]).unwrap()
  }
}

/// Shared meta info for the core and core plugins, for example, shared swc [SourceMap]
/// The **custom** field can be used for custom plugins to store shared meta data across compilation
pub struct ContextMetaData {
  // shared meta by core plugins
  pub script: ScriptContextMetaData,
  pub css: CssContextMetaData,
  pub html: HtmlContextMetaData,
  // custom meta map
  pub custom: DashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ContextMetaData {
  pub fn new() -> Self {
    Self {
      script: ScriptContextMetaData::new(),
      css: CssContextMetaData::new(),
      html: HtmlContextMetaData::new(),
      custom: DashMap::new(),
    }
  }
}

impl Default for ContextMetaData {
  fn default() -> Self {
    Self::new()
  }
}

/// Shared script meta data used for [swc]
pub struct ScriptContextMetaData {
  pub cm: Arc<SourceMap>,
  pub globals: Globals,
  pub runtime_ast: RwLock<Option<swc_ecma_ast::Module>>,
}

impl ScriptContextMetaData {
  pub fn new() -> Self {
    Self {
      cm: Arc::new(SourceMap::new(FilePathMapping::empty())),
      globals: Globals::new(),
      runtime_ast: RwLock::new(None),
    }
  }
}

impl Default for ScriptContextMetaData {
  fn default() -> Self {
    Self::new()
  }
}

pub struct CssContextMetaData {
  pub cm: Arc<SourceMap>,
  pub globals: Globals,
}

impl CssContextMetaData {
  pub fn new() -> Self {
    Self {
      cm: Arc::new(SourceMap::new(FilePathMapping::empty())),
      globals: Globals::new(),
    }
  }
}

impl Default for CssContextMetaData {
  fn default() -> Self {
    Self::new()
  }
}

pub struct HtmlContextMetaData {
  pub cm: Arc<SourceMap>,
  pub globals: Globals,
}

impl HtmlContextMetaData {
  pub fn new() -> Self {
    Self {
      cm: Arc::new(SourceMap::new(FilePathMapping::empty())),
      globals: Globals::new(),
    }
  }
}

impl Default for HtmlContextMetaData {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {

  mod add_watch_files {

    use super::super::CompilationContext;

    #[test]
    fn file_as_root_and_dep() {
      let context = CompilationContext::default();
      let vc = "./v_c".to_string();
      let vd = "./v_d".to_string();
      let a = "./a".to_string();

      context.add_watch_files(a.clone(), vec![&vc, &vd]).unwrap();

      context.add_watch_files(vc.clone(), vec![&vd]).unwrap();

      let watch_graph = context.watch_graph.read();

      assert_eq!(watch_graph.relation_roots(&vc), vec![&a]);
      let mut r = watch_graph.relation_roots(&vd);
      r.sort();
      assert_eq!(r, vec![&a, &vc]);
    }
  }
}
