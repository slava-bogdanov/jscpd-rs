use std::path::Path;

use crate::cli::FormatMappings;

const EXTENSION_FORMATS: &[(&str, &str)] = &[
    ("c", "c"),
    ("h", "c"),
    ("cc", "cpp"),
    ("cpp", "cpp"),
    ("cxx", "cpp"),
    ("hpp", "cpp"),
    ("css", "css"),
    ("go", "go"),
    ("java", "java"),
    ("js", "javascript"),
    ("es", "javascript"),
    ("es6", "javascript"),
    ("cjs", "javascript"),
    ("mjs", "javascript"),
    ("jsx", "jsx"),
    ("json", "json"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("md", "markdown"),
    ("php", "php"),
    ("py", "python"),
    ("rb", "ruby"),
    ("rs", "rust"),
    ("sh", "bash"),
    ("bash", "bash"),
    ("sql", "sql"),
    ("ts", "typescript"),
    ("cts", "typescript"),
    ("mts", "typescript"),
    ("tsx", "tsx"),
    ("vue", "vue"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("zig", "zig"),
];

const NAME_FORMATS: &[(&str, &str)] = &[
    ("Dockerfile", "docker"),
    ("GNUmakefile", "makefile"),
    ("Makefile", "makefile"),
];

pub fn format_for_path<'a>(
    path: &Path,
    formats_exts: &'a FormatMappings,
    formats_names: &'a FormatMappings,
) -> Option<&'a str> {
    let file_name = path.file_name()?.to_string_lossy();
    if !formats_names.is_empty()
        && let Some(format) = formats_names.find_format_for_value(&file_name)
    {
        return Some(format);
    }

    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if !formats_exts.is_empty() {
        return formats_exts.find_format_for_value(&ext);
    }

    for (name, format) in NAME_FORMATS {
        if file_name == *name {
            return Some(format);
        }
    }

    EXTENSION_FORMATS
        .iter()
        .find_map(|(candidate, format)| (*candidate == ext).then_some(*format))
}

pub fn supported_formats() -> Vec<&'static str> {
    let mut formats = EXTENSION_FORMATS
        .iter()
        .map(|(_, format)| *format)
        .chain(NAME_FORMATS.iter().map(|(_, format)| *format))
        .collect::<Vec<_>>();
    formats.sort_unstable();
    formats.dedup();
    formats
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::cli::FormatMappings;

    #[test]
    fn maps_module_typescript_extensions_like_upstream() {
        let formats_exts = FormatMappings::default();
        let formats_names = FormatMappings::default();

        assert_eq!(
            super::format_for_path(Path::new("index.mts"), &formats_exts, &formats_names),
            Some("typescript")
        );
        assert_eq!(
            super::format_for_path(Path::new("index.cts"), &formats_exts, &formats_names),
            Some("typescript")
        );
    }

    #[test]
    fn maps_javascript_module_extensions_like_upstream() {
        let formats_exts = FormatMappings::default();
        let formats_names = FormatMappings::default();

        assert_eq!(
            super::format_for_path(Path::new("index.es"), &formats_exts, &formats_names),
            Some("javascript")
        );
        assert_eq!(
            super::format_for_path(Path::new("index.es6"), &formats_exts, &formats_names),
            Some("javascript")
        );
    }
}
