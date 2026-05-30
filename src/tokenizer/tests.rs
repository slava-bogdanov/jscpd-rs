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
fn jsx_text_is_split_like_javascript_text() {
    let content = r#"const x = <div>Hello, "Go" this.</div>;"#;
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(values.contains(&"Hello"));
    assert!(values.contains(&","));
    assert!(values.contains(&r#""Go""#));
    assert!(values.contains(&"this"));
}

#[test]
fn jsx_text_unclosed_quote_stops_at_line_end() {
    let content = "const x = <div>\"Captured an\nerror: null\". Clicking</div>;";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(values.contains(&"\"Captured"));
    assert!(values.contains(&"error"));
    assert!(values.contains(&"null"));
    assert!(values.contains(&"\""));
    assert!(values.contains(&"."));
}

#[test]
fn jsx_dashed_identifiers_are_split_like_prism() {
    let content = r#"expect(root).toMatchRenderedOutput(<suspensey-thing src="A" />);"#;
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(values.contains(&"suspensey"));
    assert!(values.contains(&"-"));
    assert!(values.contains(&"thing"));
    assert!(!values.contains(&"suspensey-thing"));
}

#[test]
fn js_spread_token_is_operator_like_prism() {
    let content = "const next = [...items];";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let spread = tokens
        .iter()
        .find(|token| &content[token.range[0]..token.range[1]] == "...")
        .expect("spread token");

    assert_eq!(spread.hash, hash_token(TokenKind::Operator, "...", false));
}

#[test]
fn generic_tokenizer_handles_common_non_native_formats() {
    for format in ["css", "markup", "yaml", "toml", "python"] {
        let maps = tokenize_maps_for_detection("alpha beta\n  gamma", format, &Options::default());

        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].format, format);
        assert_eq!(maps[0].tokens.len(), 3);
    }
}

#[test]
fn all_supported_formats_have_a_tokenizer_smoke_path() {
    for format in crate::formats::supported_formats() {
        let content = smoke_content_for_format(format);
        let maps = tokenize_maps_for_detection(content, format, &Options::default());
        assert!(
            maps.iter().any(|map| !map.tokens.is_empty()),
            "format {format} produced no tokens"
        );
        assert!(
            maps.iter()
                .all(|map| crate::formats::supported_formats().contains(&map.format.as_str())),
            "format {format} produced an unsupported embedded map"
        );
    }
}

fn smoke_content_for_format(format: &str) -> &'static str {
    match format {
        "astro" => "---\nconst title = 'Demo';\n---\n<section>{title}</section>\n",
        "jsx" => "const view = <section>{title}</section>;\n",
        "markdown" => "# Demo\n\n```js\nconst value = 1;\n```\n",
        "markup" => "<section><span>alpha beta</span></section>\n",
        "svelte" => "<script>let title = 'Demo';</script>\n<h1>{title}</h1>\n",
        "tsx" => "const view: JSX.Element = <section>{title}</section>;\n",
        "vue" => "<template>\n  <section>{{ title }}</section>\n</template>\n",
        _ => "alpha beta gamma\nalpha beta delta\n",
    }
}

#[test]
fn haml_comment_block_is_single_comment_token() {
    let content = "%section\n  %p Same\n-# File-specific comment\n  .settings\n    %h2 Title\n";
    let tokens = tokenize_for_detection(content, "haml", &Options::default());
    let comment = tokens
        .iter()
        .find(|token| content[token.range[0]..token.range[1]].starts_with("-#"))
        .expect("haml comment token");

    assert_eq!(comment.start.line, 3);
    assert_eq!(comment.end.line, 5);
    assert_eq!(
        &content[comment.range[0]..comment.range[1]],
        "-# File-specific comment\n  .settings\n    %h2 Title"
    );
}

#[test]
fn pug_dot_block_is_single_plain_text_token() {
    let content = "style.\n  .panel {\n    color: red;\n  }\nbody\n";
    let tokens = tokenize_for_detection(content, "pug", &Options::default());
    let block = tokens
        .iter()
        .find(|token| content[token.range[0]..token.range[1]].starts_with("style."))
        .expect("pug dot block token");

    assert_eq!(block.start.line, 1);
    assert_eq!(block.end.line, 4);
    assert_eq!(
        &content[block.range[0]..block.range[1]],
        "style.\n  .panel {\n    color: red;\n  }"
    );
}

#[test]
fn markdown_fenced_javascript_emits_embedded_map() {
    let content = "# Demo\n\n```js\nfunction alpha() {\n  return 42;\n}\n```\n";
    let maps = tokenize_maps_for_detection(content, "markdown", &Options::default());

    assert!(maps.iter().any(|map| map.format == "markdown"));
    let javascript = maps
        .iter()
        .find(|map| map.format == "javascript")
        .expect("embedded javascript map");

    assert_eq!(javascript.tokens[0].start.line, 4);
    assert_eq!(javascript.tokens[0].start.column, 1);
    assert_eq!(
        &content[javascript.tokens[0].range[0]..javascript.tokens[0].range[1]],
        "function"
    );
}

#[test]
fn markdown_fenced_code_is_removed_from_markdown_map() {
    let content = "before\n\n```ts\nconst hidden = true;\n```\n\nafter\n";
    let maps = tokenize_maps_for_detection(content, "markdown", &Options::default());
    let markdown = maps
        .iter()
        .find(|map| map.format == "markdown")
        .expect("markdown map");
    let markdown_values = markdown
        .tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(markdown_values.contains(&"before"));
    assert!(markdown_values.contains(&"after"));
    assert!(!markdown_values.contains(&"hidden"));
}

#[test]
fn markdown_fenced_typescript_uses_language_name() {
    let content = "```typescript\ntype Answer = number;\n```\n";
    let maps = tokenize_maps_for_detection(content, "markdown", &Options::default());

    assert!(maps.iter().any(|map| map.format == "typescript"));
}

#[test]
fn markdown_front_matter_emits_yaml_map() {
    let content = "---\ntitle: Demo\ntags:\n  - docs\n---\n# Demo\n";
    let maps = tokenize_maps_for_detection(content, "markdown", &Options::default());
    let yaml = maps
        .iter()
        .find(|map| map.format == "yaml")
        .expect("front matter yaml map");

    assert_eq!(yaml.tokens[0].start.line, 2);
    assert_eq!(
        &content[yaml.tokens[0].range[0]..yaml.tokens[0].range[1]],
        "title"
    );
    assert_eq!(
        &content[yaml.tokens[1].range[0]..yaml.tokens[1].range[1]],
        ":"
    );
}

#[test]
fn markdown_embedded_generic_blocks_keep_whitespace_tokens() {
    let content =
        "```coffeescript\njscpd = require 'jscpd'\nresult = jscpd::run\n  reporter: json\n```\n";
    let maps = tokenize_maps_for_detection(content, "markdown", &Options::default());
    let coffeescript = maps
        .iter()
        .find(|map| map.format == "coffeescript")
        .expect("coffeescript map");

    assert!(
        coffeescript
            .tokens
            .iter()
            .any(|token| &content[token.range[0]..token.range[1]] == "\n")
    );
}

#[test]
fn markup_emits_embedded_script_and_style_maps() {
    let content = "<html>\n<script language=\"JavaScript\">\nfunction demo() { return 1; }\n</script>\n<style type=\"text/css\">\nbody { color: red; }\n</style>\n</html>\n";
    let maps = tokenize_maps_for_detection(content, "markup", &Options::default());

    assert!(maps.iter().any(|map| map.format == "markup"));
    let javascript = maps
        .iter()
        .find(|map| map.format == "javascript")
        .expect("embedded javascript map");
    let css = maps
        .iter()
        .find(|map| map.format == "css")
        .expect("embedded css map");

    assert_eq!(javascript.tokens[0].start.line, 3);
    assert_eq!(
        &content[javascript.tokens[0].range[0]..javascript.tokens[0].range[1]],
        "function"
    );
    let body = css
        .tokens
        .iter()
        .find(|token| &content[token.range[0]..token.range[1]] == "body")
        .expect("body selector token");
    assert_eq!(body.start.line, 6);
}

#[test]
fn markup_emits_inline_style_attr_css_map() {
    let content = "<h4  style=\"visibility: hidden\">Order Search</h4>\n";
    let maps = tokenize_maps_for_detection(content, "markup", &Options::default());

    let css = maps
        .iter()
        .find(|map| map.format == "css")
        .expect("inline style css map");
    let values = css
        .tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec!["  ", "style", "=\"", "visibility", ":", " hidden", "\""]
    );
    assert_eq!(css.tokens[0].start.line, 1);
    assert_eq!(css.tokens[0].start.column, 4);

    let markup = maps
        .iter()
        .find(|map| map.format == "markup")
        .expect("markup map");
    assert!(
        !markup
            .tokens
            .iter()
            .any(|token| &content[token.range[0]..token.range[1]] == "style")
    );
}

#[test]
fn markup_inline_style_attr_respects_ignore_regions() {
    let content = "<!-- jscpd:ignore-start -->\n<h4 style=\"visibility: hidden\">Order Search</h4>\n<!-- jscpd:ignore-end -->\n";
    let maps = tokenize_maps_for_detection(content, "markup", &Options::default());

    assert!(maps.iter().all(|map| map.format != "css"));
}

#[test]
fn vue_sfc_emits_template_script_and_style_maps() {
    let content = "<template>\n  <section>{{ title }}</section>\n</template>\n<style lang=\"scss\">\n.panel { color: red; }\n</style>\n<script setup lang=\"ts\">\nconst title: string = 'Demo';\n</script>\n";
    let maps = tokenize_maps_for_detection(content, "vue", &Options::default());

    assert!(maps.iter().any(|map| map.format == "markup"));
    assert!(maps.iter().any(|map| map.format == "scss"));
    let typescript = maps
        .iter()
        .find(|map| map.format == "typescript")
        .expect("typescript map");

    assert_eq!(typescript.tokens[0].start.line, 8);
    assert_eq!(
        &content[typescript.tokens[0].range[0]..typescript.tokens[0].range[1]],
        "const"
    );
}

#[test]
fn vue_sfc_trims_edge_whitespace_from_embedded_block_maps() {
    let content = "<template>\n  <section>{{ title }}</section>\n</template>\n<style lang=\"scss\">\n.panel { color: red; }\n</style>\n";
    let maps = tokenize_maps_for_detection(content, "vue", &Options::default());
    let markup = maps
        .iter()
        .find(|map| map.format == "markup")
        .expect("markup map");
    let scss = maps
        .iter()
        .find(|map| map.format == "scss")
        .expect("scss map");

    assert_eq!(markup.tokens[0].start.line, 2);
    assert_eq!(markup.tokens.last().unwrap().end.line, 2);
    assert_eq!(scss.tokens[0].start.line, 5);
    assert_eq!(scss.tokens.last().unwrap().end.line, 5);
}

#[test]
fn vue_template_emits_inline_style_attr_css_map() {
    let content = "<template>\n  <div style=\"color: red\">{{ title }}</div>\n</template>\n";
    let maps = tokenize_maps_for_detection(content, "vue", &Options::default());

    let css = maps
        .iter()
        .find(|map| map.format == "css")
        .expect("inline style css map");
    let values = css
        .tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec![" ", "style", "=\"", "color", ":", " red", "\""]
    );
    assert_eq!(css.tokens[0].start.line, 2);
    assert_eq!(css.tokens[0].start.column, 7);
}

#[test]
fn svelte_sfc_emits_markup_script_and_style_maps() {
    let content = "<script>\nlet title = 'Demo';\n</script>\n<h1>{title}</h1>\n<style>\nh1 { color: red; }\n</style>\n";
    let maps = tokenize_maps_for_detection(content, "svelte", &Options::default());

    assert!(maps.iter().any(|map| map.format == "markup"));
    assert!(maps.iter().any(|map| map.format == "javascript"));
    let css = maps
        .iter()
        .find(|map| map.format == "css")
        .expect("css map");
    let h1 = css
        .tokens
        .iter()
        .find(|token| &content[token.range[0]..token.range[1]] == "h1")
        .expect("h1 selector token");

    assert_eq!(h1.start.line, 6);
}

#[test]
fn astro_sfc_emits_frontmatter_script_style_and_markup_maps() {
    let content = "---\nconst title: string = 'Demo';\n---\n<article>{title}</article>\n<script>\nconsole.log(title);\n</script>\n<style>\narticle { color: red; }\n</style>\n";
    let maps = tokenize_maps_for_detection(content, "astro", &Options::default());

    assert!(maps.iter().any(|map| map.format == "markup"));
    assert!(maps.iter().any(|map| map.format == "javascript"));
    assert!(maps.iter().any(|map| map.format == "css"));
    let typescript = maps
        .iter()
        .find(|map| map.format == "typescript")
        .expect("frontmatter typescript map");

    assert_eq!(typescript.tokens[0].start.line, 2);
    assert_eq!(
        &content[typescript.tokens[0].range[0]..typescript.tokens[0].range[1]],
        "const"
    );
}

#[test]
fn astro_markup_trims_blanked_frontmatter_whitespace() {
    let content = "---\nconst title = 'Hello';\n---\n\n<main>{title}</main>\n";
    let maps = tokenize_maps_for_detection(content, "astro", &Options::default());
    let markup = maps
        .iter()
        .find(|map| map.format == "markup")
        .expect("markup map");

    assert_eq!(markup.tokens[0].start.line, 5);
    assert_eq!(
        &content[markup.tokens[0].range[0]..markup.tokens[0].range[1]],
        "<"
    );
}

#[test]
fn apex_soql_blocks_emit_sql_map() {
    let content = "public class A {\n  Account acc = [\n    SELECT Id\n    FROM Account\n  ];\n}\n";
    let maps = tokenize_maps_for_detection(content, "apex", &Options::default());

    assert!(maps.iter().any(|map| map.format == "apex"));
    let sql = maps
        .iter()
        .find(|map| map.format == "sql")
        .expect("sql map");
    let first = sql.tokens.first().expect("sql token");

    assert_eq!(first.start.line, 2);
    assert_eq!(&content[first.range[0]..first.range[1]], "[");
}

#[test]
fn tap_yamlish_blocks_emit_yaml_map() {
    let content = "not ok 1 - failed\n  ---\n  message: Expected value\n  actual: null\n  ...\n";
    let maps = tokenize_maps_for_detection(content, "tap", &Options::default());
    let yaml = maps
        .iter()
        .find(|map| map.format == "yaml")
        .expect("yaml map");

    assert_eq!(yaml.tokens[0].start.line, 2);
    assert_eq!(yaml.tokens[0].start.column, 3);
    assert_eq!(
        &content[yaml.tokens[0].range[0]..yaml.tokens[0].range[1]],
        "---"
    );
    assert!(maps.iter().any(|map| map.format == "tap"));
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
fn yaml_quoted_scalars_are_single_string_tokens() {
    let content = "email: \"jane@example.com\"\n";
    let tokens = tokenize_for_detection(content, "yaml", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(values, vec!["email", ":", "\"jane@example.com\""]);
}

#[test]
fn strict_mode_keeps_generic_whitespace_tokens() {
    let content = "alpha beta\ngamma";
    let strict_options = Options {
        mode: crate::cli::Mode::Strict,
        ..Options::default()
    };

    let mild = tokenize_for_detection(content, "yaml", &Options::default());
    let strict = tokenize_for_detection(content, "yaml", &strict_options);
    let token_values = strict
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(mild.len(), 3);
    assert_eq!(token_values, vec!["alpha", " ", "beta", "\n", "gamma"]);
}

#[test]
fn strict_mode_keeps_js_whitespace_tokens() {
    let content = "let a = 1;\nlet b = 2;";
    let strict_options = Options {
        mode: crate::cli::Mode::Strict,
        ..Options::default()
    };

    let mild = tokenize_for_detection(content, "javascript", &Options::default());
    let strict = tokenize_for_detection(content, "javascript", &strict_options);
    let token_values = strict
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(mild.len(), 10);
    assert!(strict.len() > mild.len());
    assert!(token_values.contains(&" "));
    assert!(token_values.contains(&"\n"));
}

#[test]
fn weak_mode_skips_generic_double_dash_comments() {
    let content = "-- first comment\nselect one\n-- second comment\nfrom table\n";
    let weak_options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };

    let strong = tokenize_for_detection(content, "sql", &Options::default());
    let weak = tokenize_for_detection(content, "sql", &weak_options);
    let token_values = weak
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(strong.len(), 6);
    assert_eq!(token_values, vec!["select", "one", "from", "table"]);
}

#[test]
fn weak_mode_skips_generic_semicolon_comments() {
    let content = "; first comment\n[main]\nkey=value\n  ; second comment\nother=value\n";
    let weak_options = Options {
        mode: crate::cli::Mode::Weak,
        ..Options::default()
    };

    let strong = tokenize_for_detection(content, "ini", &Options::default());
    let weak = tokenize_for_detection(content, "ini", &weak_options);
    let token_values = weak
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(strong.len(), 11);
    assert_eq!(
        token_values,
        vec!["[", "main", "]", "key", "=", "value", "other", "=", "value"]
    );
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
fn css_like_tokenizer_splits_punctuation() {
    let content = "#app .title { color: saturate(@base, 5%); }";
    let tokens = tokenize_for_detection(content, "css", &Options::default());
    let token_values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(
        token_values,
        vec![
            "#app", ".title", "{", "color", ":", "saturate", "(", "@base", ",", "5%", ")", ";", "}"
        ]
    );
}

#[test]
fn code_like_tokenizer_splits_punctuation_and_operators() {
    let content = "fn call<T>(x: i32) -> bool { x >= 1 }";
    let tokens = tokenize_for_detection(content, "rust", &Options::default());
    let token_values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(
        token_values,
        vec![
            "fn", "call", "<", "T", ">", "(", "x", ":", "i32", ")", "->", "bool", "{", "x", ">=",
            "1", "}"
        ]
    );
}

#[test]
fn long_tail_code_like_formats_split_punctuation_and_operators() {
    let content = "value = call(item, 1);";
    for format in [
        "aspnet",
        "cfml",
        "cfscript",
        "clojure",
        "cmake",
        "coffeescript",
        "csv",
        "dot",
        "eiffel",
        "haml",
        "ini",
        "markup",
        "ocaml",
        "plsql",
        "purescript",
        "python",
        "qsharp",
        "rescript",
        "robotframework",
        "sparql",
        "tt2",
        "yaml",
    ] {
        let tokens = tokenize_for_detection(content, format, &Options::default());
        let token_values = tokens
            .iter()
            .map(|token| &content[token.range[0]..token.range[1]])
            .collect::<Vec<_>>();

        assert_eq!(
            token_values,
            vec!["value", "=", "call", "(", "item", ",", "1", ")", ";"],
            "{format}"
        );
    }
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
fn js_line_comments_split_into_comment_tokens() {
    let content = "// really an argument\nconst a = 1;\n";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let comment_values = tokens
        .iter()
        .filter_map(|token| {
            let value = &content[token.range[0]..token.range[1]];
            value.starts_with("//").then_some(value)
        })
        .collect::<Vec<_>>();

    assert_eq!(comment_values, vec!["//"]);
    assert!(
        tokens
            .iter()
            .any(|token| &content[token.range[0]..token.range[1]] == "really")
    );
}

#[test]
fn js_hashbang_splits_like_prism() {
    let content = "#!/usr/bin/env node\n'use strict';\n";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .take(9)
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        vec!["#", "!", "/", "usr", "/", "bin", "/", "env", "node"]
    );
    assert_eq!(tokens[0].hash, hash_token(TokenKind::Default, "#", false));
    assert_eq!(tokens[1].hash, hash_token(TokenKind::Operator, "!", false));
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
fn keeps_template_interpolation_space_tokens_like_prism() {
    let content = "const x = `${store ? '[Store]' : '[No Store]'}`;";
    let tokens = tokenize_for_detection(content, "typescript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(
        values
            .windows(3)
            .any(|window| window == ["store", " ", "?"])
    );
    assert!(
        values
            .windows(3)
            .any(|window| window == ["?", " ", "'[Store]'"])
    );
    assert!(
        values
            .windows(3)
            .any(|window| window == ["'[Store]'", " ", ":"])
    );
    assert!(
        values
            .windows(3)
            .any(|window| window == [":", " ", "'[No Store]'"])
    );
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
fn js_regex_after_recoverable_parse_error_stays_single_token() {
    let content = "export type Flags = {enabled: boolean};\n\
export function normalize(str) {\n\
  return str.replace(/Check your code at .+?:\\d+/g, 'Check your code at **');\n\
}\n";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(values.contains(&"/Check your code at .+?:\\d+/g"));
    assert!(!values.windows(2).any(|window| window == ["/", "Check"]));
}

#[test]
fn typescript_array_regex_splits_like_prism() {
    let content = r#"const restrictions = [/\.css$/i];"#;
    let tokens = tokenize_for_detection(content, "typescript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(
        values
            .windows(6)
            .any(|window| window == ["/", "\\", ".", "css$", "/", "i"])
    );
    assert!(!values.contains(&r#"/\.css$/i"#));
}

#[test]
fn js_division_after_identifier_is_not_recovered_as_regex() {
    let content = "const ratio = total / count / scale;\n";
    let tokens = tokenize_for_detection(content, "javascript", &Options::default());
    let values = tokens
        .iter()
        .map(|token| &content[token.range[0]..token.range[1]])
        .collect::<Vec<_>>();

    assert!(values.contains(&"/"));
    assert!(!values.contains(&"/ count /"));
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
