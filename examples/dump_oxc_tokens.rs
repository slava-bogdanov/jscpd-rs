use std::{env, fs, path::Path};

use oxc_allocator::Allocator;
use oxc_parser::{config::TokensLexerConfig, lexer::Lexer};
use oxc_span::SourceType;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("usage: dump_oxc_tokens <file> [start-line] [end-line]");
        std::process::exit(2);
    }
    let path = &args[1];
    let start_line = args
        .get(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let end_line = args
        .get(3)
        .and_then(|value| value.parse().ok())
        .unwrap_or(usize::MAX);
    let content = fs::read_to_string(path).expect("read file");
    let source_type = SourceType::from_path(Path::new(path)).unwrap_or_default();
    let allocator = Allocator::new();
    let mut lexer = Lexer::new_for_benchmarks(&allocator, &content, source_type, TokensLexerConfig);
    let line_index = LineIndex::new(&content);
    let mut token = lexer.first_token();
    let mut count = 0usize;

    while !token.kind().is_eof() {
        let start = token.start() as usize;
        let end = token.end() as usize;
        let location = line_index.location(start);
        if location.line >= start_line && location.line <= end_line {
            println!(
                "{}:{} {:?}:{}",
                location.line,
                location.column,
                token.kind(),
                &content[start..end]
            );
        }
        count += 1;
        token = lexer.next_token_for_benchmarks();
    }
    eprintln!("tokens: {count}");
}

struct LineIndex {
    newlines: Vec<usize>,
}

impl LineIndex {
    fn new(content: &str) -> Self {
        Self {
            newlines: content
                .bytes()
                .enumerate()
                .filter_map(|(idx, byte)| (byte == b'\n').then_some(idx))
                .collect(),
        }
    }

    fn location(&self, offset: usize) -> Location {
        let previous_newlines = self
            .newlines
            .partition_point(|newline_offset| *newline_offset < offset);
        let line_start = if previous_newlines == 0 {
            0
        } else {
            self.newlines[previous_newlines - 1] + 1
        };
        Location {
            line: previous_newlines + 1,
            column: offset - line_start + 1,
        }
    }
}

struct Location {
    line: usize,
    column: usize,
}
