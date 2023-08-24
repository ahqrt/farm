use std::mem::replace;

use farmfe_core::{
  config::PartialBundlingModuleBucketsConfig,
  hashbrown::{HashMap, HashSet},
  module::{ModuleId, ModuleType},
};

use crate::ResourceUnitId;

/// A ModuleBucket is a collection of modules in the same ModuleGroup.
/// By default, a ModuleBucket is generated by following rule:
/// The modules which are in the same ModuleGroups are in the same ModuleBucket. For example, if there are two ModuleGroups A and B. if module c is in ModuleGroup A and ModuleGroup B, module d is only in ModuleGroup A, then c and d are in the different ModuleBucket.
///
/// A ModuleBucket can generate multiple ResourcePots.
#[derive(Debug)]
pub struct ModuleBucket {
  pub id: ModuleBucketId,
  modules: HashSet<ModuleId>,
  pub config: PartialBundlingModuleBucketsConfig,
  pub resource_units: HashSet<ResourceUnitId>,
  pub size: HashMap<ModuleType, usize>,
}

impl ModuleBucket {
  pub fn new(
    id: ModuleBucketId,
    modules: HashSet<ModuleId>,
    config: PartialBundlingModuleBucketsConfig,
  ) -> Self {
    Self {
      id,
      modules,
      config,
      resource_units: HashSet::new(),
      size: HashMap::new(),
    }
  }

  pub fn modules(&self) -> &HashSet<ModuleId> {
    &self.modules
  }

  pub fn resource_units(&self) -> &HashSet<ResourceUnitId> {
    &self.resource_units
  }

  fn add_size(&mut self, module_type: &ModuleType, size: usize) {
    if !self.size.contains_key(module_type) {
      self.size.insert(module_type.clone(), 0);
    }

    *self.size.get_mut(module_type).unwrap() += size;
  }

  fn sub_size(&mut self, module_type: &ModuleType, size: usize) {
    if self.size.contains_key(module_type) {
      *self.size.get_mut(module_type).unwrap() -= size;
    }
  }

  pub fn total_size(&self) -> u128 {
    self.size.values().fold(0, |r, s| r + (*s as u128))
  }

  pub fn add_module(&mut self, module_id: ModuleId, module_type: &ModuleType, size: usize) {
    self.modules.insert(module_id);
    self.add_size(module_type, size);
  }

  pub fn replace_modules(&mut self, modules: HashSet<ModuleId>) {
    self.modules = modules;
  }

  pub fn take_modules(&mut self) -> HashSet<ModuleId> {
    replace(&mut self.modules, HashSet::new())
  }

  pub fn add_resource_pot(&mut self, resource_pot_id: ResourceUnitId) {
    self.resource_units.insert(resource_pot_id);
  }

  pub fn remove_module(
    &mut self,
    module_id: &ModuleId,
    module_type: &ModuleType,
    size: usize,
  ) -> bool {
    self.sub_size(module_type, size);

    self.modules.remove(module_id)
  }
}

pub fn find_best_process_bucket(
  module_bucket_ids: &HashSet<ModuleBucketId>,
  module_bucket_map: &HashMap<ModuleBucketId, ModuleBucket>,
) -> ModuleBucketId {
  module_bucket_ids
    .iter()
    .reduce(|a, b| {
      let module_bucket_1 = module_bucket_map.get(a).unwrap();
      let module_bucket_2 = module_bucket_map.get(b).unwrap();

      let r = module_bucket_1
        .config
        .weight
        .cmp(&module_bucket_2.config.weight);
      if !r.is_eq() {
        return if r.is_gt() { a } else { b };
      }

      let a_units_len = module_bucket_1.resource_units().len() as isize;
      let b_units_len = module_bucket_2.resource_units().len() as isize;

      let r = ((module_bucket_1.total_size()) * (a_units_len as u128))
        .cmp(&(module_bucket_2.total_size() * b_units_len as u128));

      if !r.is_eq() {
        return if r.is_gt() { a } else { b };
      }

      let r = a_units_len.cmp(&b_units_len);

      if !r.is_eq() {
        return if r.is_gt() { a } else { b };
      }

      a
    })
    .unwrap()
    .clone()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleBucketId {
  id: String,
}

impl From<String> for ModuleBucketId {
  fn from(id: String) -> Self {
    Self { id }
  }
}

impl ToString for ModuleBucketId {
  fn to_string(&self) -> String {
    self.id.clone()
  }
}
