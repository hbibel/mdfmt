use super::{
    ConfigError, ConfigResolver,
    glob::{filter_matches, matcher},
    resolve::{absolute, relative_path, slash},
};
use ignore::gitignore::GitignoreBuilder;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct InputSelection {
    pub files: Vec<PathBuf>,
    pub stdin: bool,
}

fn normalize(pattern: &str) -> String {
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_some_and(|c| !"$()*+?[\\]^:".contains(*c)) {
            result.push('/');
        } else {
            result.push(ch);
        }
    }
    if let Some(rest) = result.strip_prefix('#') {
        format!("!{rest}")
    } else {
        result
    }
}

fn magic_index(pattern: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, ch) in pattern.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if "*?[{(".contains(ch) {
            return Some(i);
        }
    }
    None
}

fn unescape(pattern: &str) -> String {
    let mut result = String::new();
    let mut escaped = false;
    for ch in pattern.chars() {
        if ch == '\\' && !escaped {
            escaped = true;
        } else {
            result.push(ch);
            escaped = false;
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn walk(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect()
}

fn expanded_directory_pattern(base: &Path, pattern: &str) -> String {
    let globstar_directory = pattern.rsplit_once("**/").is_some_and(|(_, name)| {
        !name.is_empty()
            && !name.contains('/')
            && !name.contains(['*', '?', '[', ']', '{', '}'])
            && (name.starts_with('.') || Path::new(name).extension().is_none())
    });
    if globstar_directory || base.join(pattern).is_dir() {
        format!("{}/**", pattern.trim_end_matches('/'))
    } else {
        pattern.into()
    }
}

fn enumerate(base: &Path, pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
    let expanded = expanded_directory_pattern(base, pattern);
    let pattern = expanded.as_str();
    let directory_only = pattern.ends_with('/');
    let compiled = matcher(pattern.trim_end_matches('/'))?;
    if magic_index(pattern).is_none() {
        let path = absolute(base, Path::new(&unescape(pattern)));
        return Ok(if path.is_dir() {
            walk(&path)
        } else if directory_only {
            vec![]
        } else {
            vec![path]
        });
    }
    let magic = magic_index(pattern).unwrap();
    let prefix = pattern[..magic]
        .rfind('/')
        .map_or("", |slash| &pattern[..=slash]);
    let root = absolute(base, Path::new(&unescape(prefix)));
    Ok(walk(&root)
        .into_iter()
        .filter(|path| {
            if directory_only {
                return false;
            }
            let target = if Path::new(pattern).is_absolute() {
                slash(path)
            } else {
                relative_path(base, path)
            };
            compiled.is_match(target)
        })
        .collect())
}

fn excluded(base: &Path, path: &Path, patterns: &[String]) -> Result<bool, ConfigError> {
    for pattern in patterns {
        let expanded = expanded_directory_pattern(base, pattern);
        let pattern = expanded.as_str();
        let compiled = matcher(pattern.trim_end_matches('/'))?;
        let prunes_directory = pattern.ends_with("/**")
            || Path::new(pattern)
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| magic_index(s).is_none());
        for target in path
            .ancestors()
            .skip(usize::from(pattern.ends_with('/')))
            .take_while(|p| *p != base)
        {
            if target != path && !prunes_directory {
                break;
            }
            let candidate = if Path::new(pattern).is_absolute() {
                slash(target)
            } else {
                relative_path(base, target)
            };
            if compiled.is_match(candidate) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn gitignored(resolver: &ConfigResolver, path: &Path) -> Result<bool, ConfigError> {
    let base = resolver.invocation();
    let options = resolver.base_options();
    let mut root = base;
    let files = if options.boolean("gitignore") {
        for ancestor in base.ancestors() {
            if ancestor.join(".git").exists() {
                root = ancestor;
                break;
            }
        }
        if !path.starts_with(root) {
            return Ok(false);
        }
        let scoped_directory = if path.starts_with(base) {
            path.parent().unwrap()
        } else {
            base
        };
        let mut directories: Vec<_> = scoped_directory
            .ancestors()
            .take_while(|p| *p != root)
            .collect();
        directories.push(root);
        directories.reverse();
        directories
            .into_iter()
            .map(|dir| dir.join(".gitignore"))
            .filter(|p| p.is_file())
            .collect()
    } else if let Some(pattern) = options.string("gitignore") {
        enumerate(base, pattern)?
    } else {
        return Ok(false);
    };
    let mut builder = GitignoreBuilder::new(root);
    for file in files {
        if file.file_name().is_some_and(|name| name == ".env") {
            return Err(ConfigError::invalid(
                "gitignore",
                "refusing to read .env as an ignore file",
            )
            .at(&file));
        }
        if fs::metadata(&file).is_ok_and(|m| m.is_file())
            && let Some(error) = builder.add(&file)
        {
            return Err(ConfigError::invalid("gitignore", error.to_string()).at(&file));
        }
    }
    let ignores = builder
        .build()
        .map_err(|e| ConfigError::invalid("gitignore", e.to_string()))?;
    if !path.starts_with(root) {
        return Ok(false);
    }
    Ok(ignores.matched(path, false).is_ignore()
        || path
            .parent()
            .unwrap()
            .ancestors()
            .take_while(|dir| *dir != root)
            .any(|directory| ignores.matched(directory, true).is_ignore()))
}

pub fn select_inputs(
    resolver: &ConfigResolver,
    patterns: &[String],
    no_globs: bool,
) -> Result<InputSelection, ConfigError> {
    let base = resolver.invocation();
    let mut patterns: Vec<_> = patterns
        .iter()
        .map(|p| {
            if p.starts_with(':') {
                p.clone()
            } else {
                normalize(p)
            }
        })
        .collect();
    if patterns == ["."] {
        patterns[0] = "*.{md,markdown}".into();
    }
    if !no_globs {
        patterns.extend(resolver.base_options().strings("globs"));
    }
    patterns.extend(
        resolver
            .base_options()
            .strings("ignores")
            .into_iter()
            .map(|p| format!("!{p}")),
    );
    let mut positive: Vec<(String, Vec<String>)> = vec![];
    let mut literal = vec![];
    let mut stdin = false;
    let mut seen_patterns = HashSet::new();
    for pattern in patterns {
        if let Some(path) = pattern.strip_prefix(':') {
            literal.push(absolute(base, Path::new(path)));
        } else if pattern == "-" {
            stdin = true;
        } else {
            if !seen_patterns.insert(pattern.clone()) {
                continue;
            }
            // Validate before interpreting a leading ! so unsupported negative extglobs cannot become ordinary exclusions.
            matcher(&pattern)?;
            if let Some(pattern) = pattern.strip_prefix('!') {
                for (_, exclusions) in &mut positive {
                    exclusions.push(pattern.into());
                }
            } else {
                positive.push((
                    pattern
                        .strip_prefix("\\:")
                        .map_or_else(|| pattern.clone(), |rest| format!(":{rest}")),
                    vec![],
                ));
            }
        }
    }
    let mut files = vec![];
    for (pattern, negative) in positive {
        for path in enumerate(base, &pattern)? {
            if !excluded(base, &path, &negative)? && !gitignored(resolver, &path)? {
                files.push(path);
            }
        }
    }
    let literal_filters: Vec<_> = resolver
        .base_options()
        .strings("globs")
        .into_iter()
        .filter(|p| p.starts_with('!'))
        .collect();
    for path in literal {
        if literal_filters.is_empty()
            || filter_matches(&relative_path(base, &path), &literal_filters)?
        {
            files.push(path);
        }
    }
    let mut seen = HashSet::new();
    let mut selected = vec![];
    for path in files {
        if seen.insert(path.clone()) && !resolver.resolve_file(&path)?.ignored {
            selected.push(path);
        }
    }
    Ok(InputSelection {
        files: selected,
        stdin,
    })
}
