use dynamecs_analyze::{InvalidTreeLayout, SpanPath, SpanTree};

#[test]
fn try_from_depth_first_valid_trees() {
    let (paths_df, payloads) = vec![
        (span_path!("a"), "A"),
        (span_path!("a", "b"), "B"),
        (span_path!("a", "b", "c"), "C"),
        (span_path!("a", "b", "d"), "D"),
        // TODO: Reorder so that d comes before c, this should also be valid
        (span_path!("a", "b", "d", "e"), "E"),
        (span_path!("a", "f"), "F"),
    ].into_iter().unzip();
    assert!(SpanTree::try_from_depth_first(paths_df, payloads).is_ok());
}

#[test]
fn try_from_depth_first_invalid_trees() {
    {
        // Empty tree not permitted (there must be a root)
        let (paths_df, payloads): (Vec<_>, Vec<()>) = (vec![], vec![]);
        assert_eq!(SpanTree::try_from_depth_first(paths_df, payloads), Err(InvalidTreeLayout::Empty));
    }

    {

    }
}