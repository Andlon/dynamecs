use dynamecs_analyze::{SpanPath, SpanTree};

#[test]
fn span_tree_valid_trees() -> Result<(), Box<dyn std::error::Error>> {
    {
        let paths = vec![ SpanPath::new(vec![]) ];
        let payloads = vec![0];
        let tree = SpanTree::try_from_depth_first_ordering(paths, payloads)?;
        let root = tree.root();
        assert_eq!(root.path(), SpanPath::new(vec![]));
        assert_eq!(root.payload(), &0);
        assert_eq!(root.count_children(), 0);
    }

    {
        let paths = vec![ span_path!("a", "b"),
                          span_path!("a", "b", "c"),
                          span_path!("a", "b", "d"),
                          span_path!("a", "b", "d", "e")];
        let payloads = vec![ "ab", "abc", "abd", "abde" ];
        let tree = SpanTree::try_from_depth_first_ordering(paths, payloads)?;
        let ab = tree.root();
        assert_eq!(ab.payload(), &"ab");
        assert_eq!(ab.count_children(), 2);

        let abc = ab.visit_children()
            .find(|node| node.path() == span_path!("a", "b", "c"))
            .unwrap();
        let abd = ab.visit_children()
            .find(|node| node.path() == span_path!("a", "b", "d"))
            .unwrap();
        assert_eq!(abc.count_children(), 0);
        assert_eq!(abc.payload(), &"abc");
        assert_eq!(abd.count_children(), 1);
        assert_eq!(abd.payload(), &"abd");

        let abde = abd.visit_children().next().unwrap();
        assert_eq!(abde.path(), span_path!("a", "b", "d", "e"));
        assert_eq!(abde.payload(), &"abde");
    }

    Ok(())
}

#[test]
fn span_tree_invalid_trees() {
    {
        let paths = vec![ span_path!("a"), span_path!("b") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }

    {
        let paths = vec![ span_path!("a"), span_path!("a", "b", "c") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }

    {
        let paths = vec![ span_path!("a"), span_path!("a") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }

    {
        let paths = vec![ span_path!("a"), span_path!("a", "b"), span_path!("c") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }

    {
        let paths = vec![ span_path!("a"), span_path!("a") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }

    {
        let paths = vec![ span_path!("b"), span_path!("a") ];
        let payloads = vec![(); paths.len()];
        assert!(SpanTree::try_from_depth_first_ordering(paths, payloads).is_err());
    }
}