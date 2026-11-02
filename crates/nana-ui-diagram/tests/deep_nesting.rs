//! Hostile nesting: every grammar given the deepest nesting it allows
//! within the source limits parses and lays out on a 512 KiB thread,
//! returning a scene or an error, never overflowing the stack or looping.

use nana_ui_diagram::{DiagramError, DiagramMetrics, FixedMeasure, LimitKind, Limits, render};

const STACK: usize = 512 * 1024;

/// Render on a small thread; a stack overflow aborts the test binary.
fn render_small(source: String) -> Result<usize, DiagramError> {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            render(
                &source,
                &FixedMeasure,
                &DiagramMetrics::DEFAULT,
                &Limits::DEFAULT,
            )
            .map(|scene| scene.items.len())
        })
        .unwrap()
        .join()
        .unwrap()
}

fn too_deep(source: String) {
    assert_eq!(
        render_small(source),
        Err(DiagramError::TooLarge(LimitKind::ClusterDepth))
    );
}

/// One item per line, each indented one column deeper than the last, as
/// deep as fits in the source budget (about 490 levels).
fn staircase(header: &str, item: impl Fn(usize) -> String) -> String {
    let mut source = format!("{header}\n");
    for i in 0.. {
        let line = format!("{}{}\n", " ".repeat(i + 1), item(i));
        if source.len() + line.len() > Limits::DEFAULT.source_bytes {
            break;
        }
        source.push_str(&line);
    }
    source
}

/// `depth` opening lines around `inner`, then as many `close` lines.
fn nested(
    header: &str,
    depth: usize,
    open: impl Fn(usize) -> String,
    inner: &str,
    close: &str,
) -> String {
    let opens: String = (0..depth).map(open).collect();
    format!("{header}\n{opens}{inner}\n{}", close.repeat(depth))
}

#[test]
fn indentation_trees_lay_out_at_full_depth() {
    // Indentation depth is bounded only by the source budget; the tree
    // layouts walk it without recursing per level.
    for source in [
        staircase("mindmap", |i| format!("n{i}")),
        staircase("treeView-beta", |i| format!("n{i}")),
        staircase("treemap-beta", |i| format!("\"n{i}\": 1")),
        staircase("ishikawa", |i| format!("n{i}")),
        staircase("kanban", |i| format!("n{i}[t]")),
    ] {
        let _ = render_small(source);
    }
}

#[test]
fn blocks_deeper_than_the_cluster_depth_fail() {
    let depth = Limits::DEFAULT.cluster_depth + 1;
    too_deep(nested(
        "block-beta",
        depth,
        |i| format!("block:b{i}\n"),
        "a",
        "end\n",
    ));
    too_deep(nested(
        "stateDiagram-v2",
        depth,
        |i| format!("state s{i} {{\n"),
        "[*] --> a",
        "}\n",
    ));
    too_deep(nested(
        "sequenceDiagram",
        depth,
        |_| "loop x\n".into(),
        "A->>B: m",
        "end\n",
    ));
    too_deep(nested(
        "classDiagram",
        depth,
        |i| format!("namespace n{i} {{\n"),
        "class A",
        "}\n",
    ));
    too_deep(nested(
        "C4Context",
        depth,
        |i| format!("Boundary(b{i}, \"b\") {{\n"),
        "System(s, \"s\")",
        "}\n",
    ));
    too_deep(nested(
        "flowchart TD",
        290,
        |i| format!("subgraph s{i}\n"),
        "a-->b",
        "end\n",
    ));
    // Below the cluster count, use case boundaries nest as deep as written.
    let _ = render_small(nested(
        "usecase-beta",
        290,
        |i| format!("systemBoundary b{i}\n"),
        "A(\"a\")",
        "end\n",
    ));
}

#[test]
fn brackets_ten_thousand_deep_fail_cleanly() {
    too_deep(format!("zenuml\n{}", "A.m(){".repeat(10_000)));
    too_deep(format!(
        "zenuml\n{}A.m(){}",
        "if(x){".repeat(10_000),
        "}".repeat(10_000)
    ));
    too_deep(format!(
        "railroad-beta\nr = {}\"a\"{}",
        "(".repeat(10_000),
        ")".repeat(10_000)
    ));
    too_deep(format!(
        "railroad-ebnf-beta\nr = {}\"a\"{}",
        "[".repeat(10_000),
        "]".repeat(10_000)
    ));
    // A JSON table too deep is a warning, not a table.
    let json = format!(
        "usecase-beta\njson J@{{\"a\":{}1{}}}",
        "[".repeat(900),
        "]".repeat(900)
    );
    assert!(render_small(json).is_ok());
}

#[test]
fn config_ten_thousand_deep_is_skipped() {
    let init = format!(
        "%%{{init: {}1{}}}%%\nflowchart TD\nA",
        "{\"a\":".repeat(10_000),
        "}".repeat(10_000)
    );
    assert!(render_small(init).is_ok());
    let keys: Vec<String> = (0..400)
        .map(|i| format!("{}k{i}:", " ".repeat(i + 1)))
        .collect();
    let frontmatter = format!("---\nconfig:\n{}\n---\nflowchart TD\nA", keys.join("\n"));
    assert!(render_small(frontmatter).is_ok());
}
