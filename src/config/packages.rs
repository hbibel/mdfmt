use super::{ConfigError, load::expand_home};
use oxc_resolver::{EnforceExtension, ResolveOptions, Resolver};
use std::path::{Path, PathBuf};

pub(crate) fn resolve(config_path: &Path, reference: &str) -> Result<PathBuf, ConfigError> {
    let expanded = expand_home(reference)?;
    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    let exact = parent.join(&expanded);
    if exact.exists() {
        return Ok(exact);
    }
    let request = expanded.to_str().ok_or_else(|| {
        ConfigError::invalid(
            "extends",
            format!("cannot resolve non-UTF-8 configuration reference {reference}"),
        )
        .at(config_path)
    })?;
    let resolver = Resolver::new(ResolveOptions {
        extensions: vec![".js".into(), ".json".into(), ".node".into()],
        main_fields: vec!["main".into()],
        main_files: vec!["index".into()],
        modules: vec!["node_modules".into()],
        condition_names: ["node", "require", "module-sync", "node-addons", "default"]
            .into_iter()
            .map(String::from)
            .collect(),
        exports_fields: vec![vec!["exports".into()]],
        imports_fields: vec![],
        package_self: false,
        symlinks: true,
        node_path: false,
        builtin_modules: true,
        enforce_extension: EnforceExtension::Disabled,
        fully_specified: false,
        allow_package_exports_in_directory_resolve: false,
        tsconfig: None,
        alias: vec![],
        alias_fields: vec![],
        extension_alias: vec![],
        fallback: vec![],
        roots: vec![],
        prefer_relative: false,
        prefer_absolute: false,
        ..Default::default()
    });
    resolver
        .resolve(parent, request)
        .map(|resolution| resolution.full_path())
        .map_err(|error| {
            ConfigError::invalid(
                "extends",
                format!("cannot resolve installed configuration {reference}: {error}"),
            )
            .at(config_path)
        })
}
