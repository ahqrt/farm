use std::path::PathBuf;

use farmfe_core::{
  hashbrown::{HashMap, HashSet},
  module::{module_graph::ModuleGraph, module_group::ModuleGroupId},
  resource::resource_pot::{ResourcePot, ResourcePotId, ResourcePotType},
};

use crate::{
  generate_module_buckets::ModuleGroupBuckets, module_bucket::ModuleBucket, utils::try_get_filename,
};

/// Generate resource pots from module group buckets.
/// 1. create module pots from module buckets.
/// 2. merge module pots to resource pots.
pub fn generate_resource_pots(
  module_group_buckets: Vec<ModuleGroupBuckets>,
  mut module_buckets_map: HashMap<String, ModuleBucket>,
  module_graph: &ModuleGraph,
) -> Vec<ResourcePot> {
  let mut resource_pot_map = HashMap::<ResourcePotId, ResourcePot>::new();
  let mut handled_module_group_buckets = HashSet::new();
  let mut used_resource_pot_names = HashSet::new();

  for mut module_group_bucket in module_group_buckets {
    let module_group_id = module_group_bucket.module_group_id;
    let base_resource_pot_name = generate_resource_pot_name(
      module_group_id.clone(),
      &used_resource_pot_names,
      &module_graph,
    );
    used_resource_pot_names.insert(base_resource_pot_name.clone());

    // Sort the buckets to make sure it is stable.
    module_group_bucket.buckets.sort();

    for (index, module_bucket_id) in module_group_bucket.buckets.into_iter().enumerate() {
      if handled_module_group_buckets.contains(&module_bucket_id) {
        continue;
      }

      let module_bucket = module_buckets_map.get_mut(&module_bucket_id).unwrap();

      // TODO merge the modules in module bucket to module pots.

      let resource_pot_id = ResourcePotId::new(format!("{}_{}", base_resource_pot_name, index));
      let mut resource_pot = ResourcePot::new(
        resource_pot_id,
        ResourcePotType::from(module_bucket.module_type.clone()),
      );
      println!(
        "resource pot: {:?}. resource pot type: {:?}, module type: {:?}",
        resource_pot.id, resource_pot.resource_pot_type, module_bucket.module_type,
      );

      for module_id in module_bucket.modules() {
        resource_pot.add_module(module_id.clone());
      }

      resource_pot_map.insert(resource_pot.id.clone(), resource_pot);

      handled_module_group_buckets.insert(module_bucket_id);
    }
  }

  resource_pot_map
    .into_iter()
    .map(|item| item.1)
    .collect::<Vec<_>>()
}

/// Generate resource pot id from module group id.
/// 1. If module_group_id is entry module group, then the resource pot id is the name defined in config.
/// 2. If module_group_id is not entry module group, then the resource pot id is the module group id's filename(without extension).
///    If the filename is used by other resource pot, try use its parent dir util we find a unique name.
fn generate_resource_pot_name(
  module_group_id: ModuleGroupId,
  used_resource_pot_names: &HashSet<String>,
  module_graph: &ModuleGraph,
) -> String {
  if let Some(name) = module_graph.entries.get(&module_group_id) {
    return name.clone();
  }

  let mut path = PathBuf::from(module_group_id.to_string());
  let mut name = try_get_filename(path.clone());

  if !used_resource_pot_names.contains(&name) {
    return name;
  }

  while path.parent().is_some() {
    path = path.parent().unwrap().to_path_buf();
    // If the path is root, then break.
    if path.parent().is_none() {
      break;
    }

    name = format!("{}_{}", try_get_filename(path.clone()), name);

    if !used_resource_pot_names.contains(&name) {
      return name;
    }
  }

  return name;
}

#[cfg(test)]
mod tests {
  use farmfe_core::{
    hashbrown::HashSet,
    module::{module_graph::ModuleGraph, module_group::ModuleGroupId, Module},
  };

  use crate::generate_resource_pots::generate_resource_pot_name;

  #[test]
  fn test_generate_resource_pot_name() {
    let mut module_graph = ModuleGraph::new();
    let module_a = Module::new("test/src/a.html".into());
    module_graph
      .entries
      .insert(module_a.id.clone(), "a".to_string());
    module_graph.add_module(module_a);

    let mut used_resource_pot_names = HashSet::new();
    assert_eq!(
      generate_resource_pot_name(
        "test/src/a.html".into(),
        &used_resource_pot_names,
        &module_graph
      ),
      "a"
    );

    let group_id: ModuleGroupId = "test/src/api.ts".into();
    assert_eq!(
      generate_resource_pot_name(group_id.clone(), &used_resource_pot_names, &module_graph),
      "api"
    );

    used_resource_pot_names.insert("api".to_string());
    assert_eq!(
      generate_resource_pot_name(group_id.clone(), &used_resource_pot_names, &module_graph),
      "src_api"
    );

    used_resource_pot_names.insert("src_api".to_string());
    used_resource_pot_names.insert("test_src_api".to_string());
    assert_eq!(
      generate_resource_pot_name(group_id, &used_resource_pot_names, &module_graph),
      "test_src_api"
    );
  }
}
