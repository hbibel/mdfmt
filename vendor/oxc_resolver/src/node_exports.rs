use std::path::{Path, PathBuf};

use crate::ResolveError;

fn invalid_segment(value: &str) -> bool {
    value.split(['/', '\\']).any(|segment| {
        let decoded: Vec<u8> = percent_encoding::percent_decode_str(segment).collect();
        decoded == b"." || decoded == b".." || decoded.eq_ignore_ascii_case(b"node_modules")
    })
}

fn encoded_separator(value: &str) -> bool {
    value.as_bytes().windows(3).any(|bytes| {
        bytes[0] == b'%'
            && (bytes[1..].eq_ignore_ascii_case(b"2f") || bytes[1..].eq_ignore_ascii_case(b"5c"))
    })
}

fn validate(
    package: &Path,
    key: &str,
    target: &str,
    pattern: Option<&str>,
) -> Result<(), ResolveError> {
    let metadata = package.join("package.json");
    let Some(relative) = target.strip_prefix("./") else {
        return Err(ResolveError::InvalidPackageTarget(target.into(), key.into(), metadata));
    };
    // Filesystem components and URL parsing erase these forbidden segments.
    if invalid_segment(relative) {
        return Err(ResolveError::InvalidPackageTarget(target.into(), key.into(), metadata));
    }
    if let Some(pattern) = pattern
        && invalid_segment(pattern)
    {
        return Err(ResolveError::InvalidModuleSpecifier(pattern.into(), metadata));
    }
    let pathname = target.split(['?', '#']).next().unwrap_or(target);
    if encoded_separator(pathname) {
        return Err(ResolveError::InvalidModuleSpecifier(target.into(), metadata));
    }
    Ok(())
}

pub(crate) fn resolve(
    package: &Path,
    key: &str,
    target: &str,
    pattern: Option<&str>,
) -> Result<PathBuf, ResolveError> {
    validate(package, key, target, pattern)?;
    let metadata = package.join("package.json");
    let base = url::Url::from_directory_path(package)
        .map_err(|_| ResolveError::PathNotSupported(package.into()))?;
    let mut resolved = base.join(target).map_err(|_| {
        ResolveError::InvalidPackageTarget(target.into(), key.into(), metadata.clone())
    })?;
    if resolved.scheme() != base.scheme()
        || resolved.host_str() != base.host_str()
        || !resolved.path().starts_with(base.path())
    {
        return Err(ResolveError::InvalidPackageTarget(target.into(), key.into(), metadata));
    }
    if let Some(pattern) = pattern {
        resolved = url::Url::parse(&resolved.as_str().replace('*', pattern))
            .map_err(|_| ResolveError::InvalidModuleSpecifier(pattern.into(), metadata.clone()))?;
    }
    let pathname = resolved.path();
    let invalid_percent = pathname.as_bytes().iter().enumerate().any(|(index, byte)| {
        *byte == b'%'
            && !pathname
                .as_bytes()
                .get(index + 1..index + 3)
                .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit))
    });
    if encoded_separator(pathname)
        || invalid_percent
        || percent_encoding::percent_decode_str(pathname).decode_utf8().is_err()
    {
        return Err(ResolveError::InvalidModuleSpecifier(target.into(), metadata));
    }
    resolved.to_file_path().map_err(|_| ResolveError::PathNotSupported(package.into()))
}
