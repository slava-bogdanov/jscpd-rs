use super::*;
use crate::cli::Options;

#[test]
fn tokenizes_non_whitespace_tokens_with_locations() {
    let tokens =
        tokenize_for_detection("let a = 1;\nlet b = 2;", "javascript", &Options::default());
    assert_eq!(tokens[0].start.line, 1);
    assert_eq!(tokens[5].start.line, 2);
}

#[test]
fn skips_ignore_regions() {
    let content = "keep\n// jscpd:ignore-start\nskip\n// jscpd:ignore-end\nkeep2\n";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    assert_eq!(tokens.len(), 2);
}

#[test]
fn detection_tokenizer_avoids_token_value_allocations() {
    let tokens =
        tokenize_for_detection("let a = 1;\nlet b = 2;", "javascript", &Options::default());
    assert_eq!(tokens.len(), 10);
    assert_eq!(tokens[0].start.line, 1);
    assert_eq!(tokens[5].start.line, 2);
}

#[test]
fn js_like_json_report_positions_count_prism_whitespace_tokens() {
    let options = Options {
        reporters: vec!["json".to_string()],
        ..Options::default()
    };
    for format in ["javascript", "typescript", "jsx", "tsx"] {
        let tokens = tokenize_for_detection("let a = 1;\nlet b = 2;", format, &options);
        assert_eq!(tokens[0].start.position, 0);
        assert_eq!(tokens[1].start.position, 2);
        assert_eq!(tokens[5].start.position, 9);
    }
}

#[test]
fn jsx_attribute_expression_emits_embedded_javascript_map() {
    let maps = tokenize_maps_for_detection(
        "const x = <div className={classNames(className, classes)} />;",
        "jsx",
        &Options::default(),
    );
    assert_eq!(maps.len(), 2);
    assert_eq!(maps[0].format, "jsx");
    assert_eq!(maps[1].format, "javascript");

    let embedded = &maps[1].tokens;
    assert_eq!(embedded.len(), 9);
    assert_eq!(
        embedded.last().unwrap().end.position - embedded.first().unwrap().start.position,
        8
    );
}

#[test]
fn jsx_embedded_javascript_keeps_nested_object_whitespace() {
    let content = "const x = <A p={{\n  color: PRIMARY_COLOR\n}} />;";
    let maps = tokenize_maps_for_detection(content, "tsx", &Options::default());
    let embedded = maps
        .iter()
        .find(|map| map.format == "javascript")
        .expect("embedded javascript map");

    assert!(
        embedded
            .tokens
            .iter()
            .any(|token| &content[token.range[0]..token.range[1]] == "\n  ")
    );
}

#[test]
fn generic_tokenizer_handles_common_non_native_formats() {
    for format in ["css", "markup", "yaml", "toml", "vue"] {
        let maps = tokenize_maps_for_detection("alpha beta\n  gamma", format, &Options::default());

        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].format, format);
        assert_eq!(maps[0].tokens.len(), 3);
    }
}

#[test]
fn weak_mode_skips_generic_comments() {
    let content = "# first comment\nalpha beta\n// second comment\ngamma\n";
    let weak_options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };

    let strong = tokenize_for_detection(content, "yaml", &Options::default());
    let weak = tokenize_for_detection(content, "yaml", &weak_options);

    assert_eq!(strong.len(), 5);
    assert_eq!(weak.len(), 3);
}

#[test]
fn generic_css_ids_are_not_treated_as_hash_comments() {
    let options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };
    let tokens = tokenize_for_detection("#app .title\n", "css", &options);

    assert_eq!(tokens.len(), 2);
}

#[test]
fn weak_mode_skips_js_comments() {
    let options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };
    let strong = tokenize_for_detection(
        "const a = 1; // comment\n",
        "javascript",
        &Options::default(),
    );
    let weak = tokenize_for_detection("const a = 1; // comment\n", "javascript", &options);
    assert!(strong.len() > weak.len());
}

#[test]
fn splits_template_interpolation_like_prism() {
    let tokens = tokenize_for_detection(
        "const x = `a${b}c${d}e`;",
        "typescript",
        &Options::default(),
    );
    assert_eq!(tokens.len(), 13);
    assert_eq!(tokens[3].start.column, 11);
    assert_eq!(tokens[4].start.column, 13);
    assert_eq!(tokens[6].start.column, 16);
    assert_eq!(tokens[8].start.column, 18);
    assert_eq!(tokens[10].start.column, 21);
    assert_eq!(tokens[11].start.column, 22);
}

#[test]
fn splits_optional_chaining_like_prism() {
    let tokens = tokenize_for_detection("a?.b", "typescript", &Options::default());
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[1].start.column, 2);
    assert_eq!(tokens[2].start.column, 3);
    assert_eq!(tokens[3].start.column, 4);
}

#[test]
fn merges_adjacent_generic_closing_angles_like_prism() {
    let tokens = tokenize_for_detection("type A = X<Y<Z>>;", "typescript", &Options::default());
    assert_eq!(tokens.len(), 10);
    assert_eq!(tokens[8].start.column, 15);
    assert_eq!(tokens[8].end.column, 17);
    assert_eq!(tokens[9].start.column, 17);
}

#[test]
fn weak_mode_skips_generic_markup_comments() {
    let content = "<!-- comment -->\nalpha beta\n<!-- another -->\ngamma\n";
    let weak_options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };

    let strong = tokenize_for_detection(content, "markup", &Options::default());
    let weak = tokenize_for_detection(content, "markup", &weak_options);

    assert_eq!(strong.len(), 5);
    assert_eq!(weak.len(), 3);
    let token_values: Vec<&str> = weak
        .iter()
        .map(|t| &content[t.range[0]..t.range[1]])
        .collect();
    assert_eq!(token_values, vec!["alpha", "beta", "gamma"]);
}
