use crate::cli::Options;
use crate::files::SourceFile;
use crate::tokenizer::Location;

use super::{CloneMatch, Fragment, dedup_exact_clones, detect};

#[test]
fn detects_cross_file_duplicates() {
    let options = Options {
        min_tokens: 3,
        min_lines: 0,
        ..Options::default()
    };
    let content = "alpha beta gamma delta epsilon\n";
    let files = vec![
        source("a.js", content),
        source("b.js", &format!("prefix\n{content}\nsuffix\n")),
    ];

    let result = detect(files, &options);

    assert!(!result.clones.is_empty());
}

#[test]
fn detects_generic_format_duplicates() {
    let options = Options {
        min_tokens: 3,
        min_lines: 0,
        ..Options::default()
    };
    let content = "alpha beta gamma delta epsilon\n";
    let files = vec![
        source_with_format("a.css", "css", content),
        source_with_format("b.css", "css", &format!("prefix\n{content}\nsuffix\n")),
    ];

    let result = detect(files, &options);

    assert!(!result.clones.is_empty());
}

#[test]
fn skip_local_skips_clones_inside_same_configured_root() {
    let options = Options {
        paths: vec!["project".into()],
        skip_local: true,
        min_tokens: 3,
        min_lines: 0,
        ..Options::default()
    };
    let content = "alpha beta gamma delta epsilon\n";
    let files = vec![
        source("project/dir1/a.js", content),
        source("project/dir2/b.js", content),
    ];

    let result = detect(files, &options);

    assert!(result.clones.is_empty());
}

#[test]
fn skip_local_keeps_clones_across_configured_roots() {
    let options = Options {
        paths: vec!["left".into(), "right".into()],
        skip_local: true,
        min_tokens: 3,
        min_lines: 0,
        ..Options::default()
    };
    let content = "alpha beta gamma delta epsilon\n";
    let files = vec![source("left/a.js", content), source("right/b.js", content)];

    let result = detect(files, &options);

    assert!(!result.clones.is_empty());
}

#[test]
fn skips_empty_token_sources_in_statistics() {
    let content = "// jscpd:ignore-start\nignored\n// jscpd:ignore-end\n";

    let result = detect(vec![source("ignored.js", content)], &Options::default());

    assert_eq!(result.sources.len(), 0);
    assert_eq!(result.statistics.total.sources, 0);
}

#[test]
fn detects_typescript_template_tail_clone_before_member_name_difference() {
    let options = Options {
        min_tokens: 50,
        min_lines: 5,
        ..Options::default()
    };
    let content = r#"
function first(workUnitAsyncStorage, reportResult) {
  console.log = function (...args: Array<any>) {
    const store = workUnitAsyncStorage.getStore()
    reportResult({
      type: 'console-call',
      method: 'log',
      input: `${store ? '[Store]' : '[No Store]'}: ${args.join(' ')}`,
    })
  }

  require('next/dist/server/node-environment-extensions/console-exit')

  workUnitAsyncStorage.run({ type: 'request' } as WorkUnitStore, () => {
    console.log('inside')
  })
}

function second(workUnitAsyncStorage, reportResult) {
  console.error = function (...args: Array<any>) {
    const store = workUnitAsyncStorage.getStore()
    reportResult({
      type: 'console-call',
      method: 'error',
      input: `${store ? '[Store]' : '[No Store]'}: ${args.join(' ')}`,
    })
  }

  require('next/dist/server/node-environment-extensions/console-exit')

  workUnitAsyncStorage.run({ type: 'request' } as WorkUnitStore, () => {
    console.error('inside')
  })
}
"#;

    let result = detect(
        vec![source_with_format("console.ts", "typescript", content)],
        &options,
    );

    assert!(result.clones.iter().any(|clone| {
        clone.duplication_a.start.line <= 24
            && clone.duplication_a.end.line >= 32
            && clone.duplication_b.start.line <= 7
            && clone.duplication_b.end.line >= 15
    }));
}

#[test]
fn deduplicates_exact_clone_records() {
    let mut clones = vec![
        clone_with_lines("javascript", "a.js", 1, 5, "b.js", 1, 5),
        clone_with_lines("javascript", "a.js", 1, 5, "b.js", 1, 5),
        clone_with_lines("javascript", "a.js", 6, 10, "b.js", 6, 10),
    ];

    dedup_exact_clones(&mut clones);

    assert_eq!(clones.len(), 2);
    assert_eq!(clones[0].duplication_a.start.line, 1);
    assert_eq!(clones[1].duplication_a.start.line, 6);
}

fn source(path: &str, content: &str) -> SourceFile {
    source_with_format(path, "javascript", content)
}

fn source_with_format(path: &str, format: &str, content: &str) -> SourceFile {
    SourceFile {
        source_id: path.to_string(),
        format: format.to_string(),
        content: content.to_string(),
    }
}

fn clone_with_lines(
    format: &str,
    source_a: &str,
    start_a: usize,
    end_a: usize,
    source_b: &str,
    start_b: usize,
    end_b: usize,
) -> CloneMatch {
    CloneMatch {
        format: format.to_string(),
        duplication_a: fragment(source_a, start_a, end_a),
        duplication_b: fragment(source_b, start_b, end_b),
        tokens: 20,
    }
}

fn fragment(source_id: &str, start: usize, end: usize) -> Fragment {
    Fragment {
        source_id: source_id.to_string(),
        start: location(start, 1, start),
        end: location(end, 1, end),
        range: [start, end],
        blame: None,
    }
}

fn location(line: usize, column: usize, position: usize) -> Location {
    Location {
        line,
        column,
        position,
    }
}
