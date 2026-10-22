use super::{
    ConfigError, ConfigResolver,
    glob::{Traversal, filter_matches, matcher},
    resolve::{absolute, relative_path, slash},
};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
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
    let (pattern, mut result) = pattern
        .strip_prefix(r"\:")
        .map_or((pattern, String::new()), |rest| (rest, r"\:".into()));
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_none_or(|c| !"$()*+?[]^".contains(*c)) {
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
    let dynamic = magic_index(&expanded).is_some();
    let mut files = Vec::new();
    let mut patterns = super::brace::enumerate(&expanded)?;
    patterns.sort_by_key(|pattern| pattern.encode_utf16().count());
    for pattern in patterns {
        if !pattern.is_empty() {
            files.extend(enumerate_expanded(base, &pattern, dynamic)?);
        }
    }
    files.sort();
    Ok(files)
}

fn enumerate_expanded(
    base: &Path,
    pattern: &str,
    dynamic: bool,
) -> Result<Vec<PathBuf>, ConfigError> {
    let directory_only = pattern.ends_with('/');
    let compiled = matcher(pattern.trim_end_matches('/'))?;
    if magic_index(pattern).is_none() || !super::glob::dynamic(pattern) {
        let path = absolute(base, Path::new(&unescape(pattern)));
        let target = if Path::new(pattern).is_absolute() {
            slash(&path)
        } else {
            relative_path(base, &path)
        };
        if path.is_file() && !compiled.is_match(target)? {
            return Ok(vec![]);
        }
        return Ok(if dynamic {
            if !directory_only && path.is_file() {
                vec![path]
            } else {
                vec![]
            }
        } else if path.is_dir() {
            walk(&path)
        } else {
            vec![path]
        });
    }
    let root = absolute(base, Path::new(&super::glob::parent(pattern)));
    let traversal = Traversal::new(pattern)?;
    let mut files = Vec::new();
    let mut traversal_error = None;
    let entries = WalkDir::new(&root)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if traversal_error.is_some() {
                return false;
            }
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                return true;
            }
            let target = if Path::new(pattern).is_absolute() {
                slash(entry.path())
            } else {
                relative_path(base, entry.path())
            };
            match traversal.permits(&target) {
                Ok(permitted) => permitted,
                Err(error) => {
                    traversal_error = Some(error);
                    false
                }
            }
        });
    for entry in entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.into_path();
        if directory_only {
            continue;
        }
        let target = if Path::new(pattern).is_absolute() {
            slash(&path)
        } else {
            relative_path(base, &path)
        };
        if !compiled.is_match(target)? {
            continue;
        }
        files.push(path);
    }
    if let Some(error) = traversal_error {
        return Err(error);
    }
    Ok(files)
}

fn excluded(base: &Path, path: &Path, patterns: &[String]) -> Result<bool, ConfigError> {
    for pattern in patterns {
        let pattern = pattern.strip_prefix('!').unwrap_or(pattern);
        let expanded = expanded_directory_pattern(base, pattern);
        let pattern = expanded.as_str();
        let compiled = matcher(pattern.trim_end_matches('/'))?;
        let prunes_directory = !compiled.negated
            && (pattern.ends_with("/**")
                || Path::new(pattern)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| magic_index(s).is_none()));
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
            if compiled.is_match(candidate)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ignore_search_excluded(base: &Path, path: &Path) -> bool {
    path.strip_prefix(base).is_ok_and(|relative| {
        relative.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some("node_modules" | "flow-typed" | "coverage" | ".git")
            )
        })
    })
}

fn discover_ignore_files(base: &Path, pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for pattern in super::brace::enumerate(pattern)? {
        for file in discover_ignore_expanded(base, &pattern)? {
            if seen.insert(file.clone()) {
                files.push(file);
            }
        }
    }
    files.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then(a.cmp(b))
    });
    Ok(files)
}

fn discover_ignore_expanded(base: &Path, pattern: &str) -> Result<Vec<PathBuf>, ConfigError> {
    let compiled = matcher(pattern)?;
    if compiled.negated || pattern.ends_with('/') {
        return Ok(vec![]);
    }
    if magic_index(pattern).is_none() || !super::glob::dynamic(pattern) {
        let path = absolute(base, Path::new(&unescape(pattern)));
        let target = if Path::new(pattern).is_absolute() {
            slash(&path)
        } else {
            relative_path(base, &path)
        };
        return Ok(
            if path.is_file()
                && !ignore_search_excluded(base, &path)
                && compiled.is_match(target)?
            {
                vec![path]
            } else {
                vec![]
            },
        );
    }
    let root = absolute(base, Path::new(&super::glob::parent(pattern)));
    let traversal = Traversal::new(pattern)?;
    let mut traversal_error = None;
    let mut files: Vec<_> = WalkDir::new(root)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if traversal_error.is_some() || ignore_search_excluded(base, entry.path()) {
                return false;
            }
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                return true;
            }
            let target = if Path::new(pattern).is_absolute() {
                slash(entry.path())
            } else {
                relative_path(base, entry.path())
            };
            match traversal.permits(&target) {
                Ok(permitted) => permitted,
                Err(error) => {
                    traversal_error = Some(error);
                    false
                }
            }
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .map(|path| {
            let target = if Path::new(pattern).is_absolute() {
                slash(&path)
            } else {
                relative_path(base, &path)
            };
            Ok(compiled.is_match(target)?.then_some(path))
        })
        .collect::<Result<Vec<_>, ConfigError>>()?
        .into_iter()
        .flatten()
        .collect();
    if let Some(error) = traversal_error {
        return Err(error);
    }
    files.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then(a.cmp(b))
    });
    Ok(files)
}

fn ignore_file(file: &Path) -> Result<Gitignore, ConfigError> {
    if file.file_name().is_some_and(|name| name == ".env") {
        return Err(
            ConfigError::invalid("gitignore", "refusing to read .env as an ignore file").at(file),
        );
    }
    let text = fs::read_to_string(file)
        .map_err(|e| ConfigError::invalid("gitignore", e.to_string()).at(file))?;
    let mut builder = GitignoreBuilder::new(file.parent().unwrap());
    builder
        .case_insensitive(true)
        .map_err(|e| ConfigError::invalid("gitignore", e.to_string()).at(file))?;
    for line in text.lines() {
        let line = line.strip_prefix('\u{feff}').unwrap_or(line);
        // globset treats braces as alternatives, but gitignore treats them literally.
        let mut pattern = String::new();
        let mut escaped = false;
        for ch in line.chars() {
            if !escaped && matches!(ch, '{' | '}') {
                pattern.push('\\');
            }
            pattern.push(ch);
            escaped = ch == '\\' && !escaped;
        }
        builder
            .add_line(Some(file.into()), &pattern)
            .map_err(|e| ConfigError::invalid("gitignore", e.to_string()).at(file))?;
    }
    builder
        .build()
        .map_err(|e| ConfigError::invalid("gitignore", e.to_string()).at(file))
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
            .filter(|p| p.is_file() && !ignore_search_excluded(base, p))
            .collect()
    } else if let Some(pattern) = options.string("gitignore") {
        discover_ignore_files(base, pattern)?
    } else {
        return Ok(false);
    };
    let mut ignores = vec![];
    for file in files {
        ignores.push((file.parent().unwrap().to_path_buf(), ignore_file(&file)?));
    }
    if !path.starts_with(root) {
        return Ok(false);
    }
    let matched = |candidate: &Path, directory| {
        ignores
            .iter()
            .rev()
            .find_map(|(scope, matcher)| {
                if !candidate.starts_with(scope) {
                    return None;
                }
                let result = matcher.matched(candidate, directory);
                (!result.is_none()).then(|| result.is_ignore())
            })
            .unwrap_or(false)
    };
    Ok(matched(path, false)
        || path
            .parent()
            .unwrap()
            .ancestors()
            .take_while(|dir| *dir != root)
            .any(|directory| matched(directory, true)))
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
