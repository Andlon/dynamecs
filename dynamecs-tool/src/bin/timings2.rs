use std::error::Error;
use dynamecs_analyze::iterate_records;
use dynamecs_analyze::{SpanTree, SpanTreeNode};

fn recursively_print(node: &SpanTreeNode<'_, ()>, indent: usize) {
    let indentation = " ".repeat(indent);
    println!("{indentation}{}", node.path());
    dbg!(node.count_children());
    for child in node.visit_children() {
        recursively_print(&child, indent + 2)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(arg) = std::env::args().skip(1).next() {
        let records: Vec<_> = iterate_records(&arg)?.collect::<Result<_, _>>()?;
        let paths: Vec<_> = records.iter()
            .map(|record| record.span_path())
            .collect();
        let payloads = vec![(); paths.len()];
        let tree = SpanTree::from_paths_and_payloads(paths, payloads);
        // let root = tree.root().unwrap();
        // recursively_print(&root, 0);

        Ok(())
    } else {
        Err(Box::from("missing path to log file"))
    }
}