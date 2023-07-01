#![deny(clippy::all)]

use std::path::Path;

use farmfe_core::{config::Config, module::ModuleType, plugin::Plugin, serde_json};
use farmfe_macro_plugin::farm_plugin;
use farmfe_toolkit::{fs, regex::Regex};
use grass;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[farm_plugin]
pub struct FarmPluginSass {
  sass_options: grass::Options,
  regex: Regex,
}

impl FarmPluginSass {
  pub fn new(config: &Config, options: String) -> Self {
    Self {
      sass_options: self.get_sass_options(options, config.root.clone()),
      regex: Regex::new(r#"\.(sass|scss)$"#).unwrap(),
    }
  }

  pub fn get_sass_options(&self, options: String, root: String) -> grass::Options {
    let options: Value = serde_json::from_str(&self.sass_options).unwrap_or_default();
    let mut sass_options = grass::Options::default();

    if let Value::Bool(quiet) = options.get("quiet").unwrap_or(&Value::Null) {
      sass_options = sass_options.quiet(*quiet);
    }

    if let Value::Bool(allows_charset) = options.get("allows_charset").unwrap_or(&Value::Null) {
      sass_options = sass_options.allows_charset(*allows_charset);
    }

    if let Value::Bool(unicode_error_messages) = options
      .get("unicode_error_messages")
      .unwrap_or(&Value::Null)
    {
      sass_options = sass_options.unicode_error_messages(*unicode_error_messages);
    }

    let mut paths = vec![Path::new(&root)];

    if let Value::Array(load_paths) = options.get("load_paths").unwrap_or(&Value::Null) {
      for path in load_paths {
        if let Value::String(path) = path {
          paths.push(Path::new(path));
        }
      }
    }

    sass_options = sass_options.load_paths(&paths);
    sass_options
  }
}

impl Plugin for FarmPluginSass {
  fn name(&self) -> &str {
    "FarmPluginSass"
  }

  fn load(
    &self,
    param: &farmfe_core::plugin::PluginLoadHookParam,
    _context: &std::sync::Arc<farmfe_core::context::CompilationContext>,
    _hook_context: &farmfe_core::plugin::PluginHookContext,
  ) -> farmfe_core::error::Result<Option<farmfe_core::plugin::PluginLoadHookResult>> {
    if self.regex.is_match(param.resolved_path) {
      let content = fs::read_file_utf8(param.resolved_path).unwrap();
      return Ok(Some(farmfe_core::plugin::PluginLoadHookResult {
        content,
        module_type: ModuleType::Custom(String::from("sass")),
      }));
    }
    Ok(None)
  }

  fn transform(
    &self,
    param: &farmfe_core::plugin::PluginTransformHookParam,
    _context: &std::sync::Arc<farmfe_core::context::CompilationContext>,
  ) -> farmfe_core::error::Result<Option<farmfe_core::plugin::PluginTransformHookResult>> {
    if param.module_type == ModuleType::Custom(String::from("sass")) {
      let css = grass::from_string(&param.content.to_owned(), &self.sass_options).map_err(|e| {
        farmfe_core::error::CompilationError::TransformError {
          resolved_path: param.resolved_path.to_string(),
          msg: e.to_string(),
        }
      })?;
      return Ok(Some(farmfe_core::plugin::PluginTransformHookResult {
        content: css,
        source_map: None,
        module_type: Some(farmfe_core::module::ModuleType::Css),
      }));
    }
    Ok(None)
  }
}
