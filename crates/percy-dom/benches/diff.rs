//! Native Criterion benchmarks for the virtual DOM `diff` algorithm.
//!
//! `diff` is pure (it turns two `VirtualNode` trees into a `Vec<Patch>` and never
//! touches the real DOM), so it runs on the host target without a browser. `patch`
//! cannot be benchmarked here because applying patches requires `web_sys`; benchmark
//! that path with `wasm-bindgen-test` in a headless browser instead.
//!
//! Run, saving a baseline to compare against later:
//!
//!     cargo bench -p percy-dom --bench diff -- --save-baseline main
//!
//! After making a change, compare against that baseline (this is the regression check):
//!
//!     cargo bench -p percy-dom --bench diff -- --baseline main

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use percy_dom::{AttributeValue, VirtualNode};

/// Number of rows in the "large list" scenarios. Mirrors the order of magnitude used
/// by js-framework-benchmark so the numbers are comparable to other vdom work.
const ROWS: usize = 1_000;

/// Build a `<div class="row"><span>{id}</span><span>{label}</span></div>` node.
fn row(id: usize, label: &str) -> VirtualNode {
    let mut el = VirtualNode::element("div");
    {
        let el = el.as_velement_mut().unwrap();
        el.attrs
            .insert("class".to_string(), AttributeValue::String("row".to_string()));

        let mut id_span = VirtualNode::element("span");
        id_span
            .as_velement_mut()
            .unwrap()
            .children
            .push(VirtualNode::text(id.to_string()));

        let mut label_span = VirtualNode::element("span");
        label_span
            .as_velement_mut()
            .unwrap()
            .children
            .push(VirtualNode::text(label.to_string()));

        el.children.push(id_span);
        el.children.push(label_span);
    }
    el
}

/// Build a `<div id="container"> ...rows... </div>` table of `n` rows. `label` produces
/// the second-column text for each row index, so callers can vary content between trees.
fn table(n: usize, label: impl Fn(usize) -> String) -> VirtualNode {
    let mut container = VirtualNode::element("div");
    let el = container.as_velement_mut().unwrap();
    el.attrs.insert(
        "id".to_string(),
        AttributeValue::String("container".to_string()),
    );
    el.children = (0..n).map(|i| row(i, &label(i))).collect();
    container
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff");

    // Create: an empty container becomes a full 1000-row table.
    let empty = table(0, |_| String::new());
    let full = table(ROWS, |i| format!("item {i}"));
    group.bench_function("create_1000_rows", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&empty), black_box(&full))))
    });

    // Remove: the inverse — a full table becomes empty.
    group.bench_function("remove_1000_rows", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&empty))))
    });

    // No-op: identical trees. This is the common idle-render case and should be cheap;
    // a regression here means diff is doing work when nothing changed.
    let same = table(ROWS, |i| format!("item {i}"));
    group.bench_function("noop_1000_rows", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&same))))
    });

    // Update all text: every row's label changes (text-patch heavy, structure stable).
    let all_changed = table(ROWS, |i| format!("changed {i}"));
    group.bench_function("update_all_text", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&all_changed))))
    });

    // Update every 10th row: the sparse-update case typical of real interactions.
    let every_10th = table(ROWS, |i| {
        if i % 10 == 0 {
            format!("changed {i}")
        } else {
            format!("item {i}")
        }
    });
    group.bench_function("update_every_10th", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&every_10th))))
    });

    // Append: 1000 rows grow to 1100 (children added at the tail).
    let appended = table(ROWS + 100, |i| format!("item {i}"));
    group.bench_function("append_100_rows", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&appended))))
    });

    // Remove one row from a 1000-row list (-> 999). This is the realistic partial-removal
    // case: ~999 children stay (matched by implicit key) while one is deleted, which is
    // exactly what exercises the per-child job-drain in the full reconciliation path.
    let removed_one = table(ROWS - 1, |i| format!("item {i}"));
    group.bench_function("remove_one_of_1000", |b| {
        b.iter(|| black_box(percy_dom::diff(black_box(&full), black_box(&removed_one))))
    });

    group.finish();
}

criterion_group!(benches, bench_diff);
criterion_main!(benches);
