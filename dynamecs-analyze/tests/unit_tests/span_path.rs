use dynamecs_analyze::SpanPath;


#[test]
fn is_parent_of() {
    let root = span_path!();
    let a = span_path!("a");
    let b = span_path!("b");
    let ab = span_path!("a", "b");
    let ac = span_path!("a", "c");
    let bc = span_path!("b", "c");

    {
        assert!(root.is_parent_of(&span_path!("a")));

        assert!(!root.is_parent_of(&span_path!("a", "b")));
        assert!(!root.is_parent_of(&root));
    }

    {
        assert!(a.is_parent_of(&ab));
        assert!(a.is_parent_of(&ac));

        assert!(!a.is_parent_of(&a));
        assert!(!a.is_parent_of(&b));
        assert!(!a.is_parent_of(&bc));
        assert!(!a.is_parent_of(&span_path!("aa")));
        assert!(!a.is_parent_of(&span_path!("aa", "b")));
    }

    {
        assert!(ab.is_parent_of(&span_path!("a", "b", "c")));

        assert!(!ab.is_parent_of(&span_path!("a", "b", "c", "d")));
        assert!(!ab.is_parent_of(&span_path!("a", "b", "c", "d", "e")));
        assert!(!ab.is_parent_of(&span_path!("a", "b")));
        assert!(!ab.is_parent_of(&span_path!("a")));
        assert!(!ab.is_parent_of(&span_path!("b")));
        assert!(!ab.is_parent_of(&span_path!("b", "c")));
    }
}

#[test]
fn is_ancestor_of() {
    let root = span_path!();
    let a = span_path!("a");
    let b = span_path!("b");
    let ab = span_path!("a", "b");
    let ac = span_path!("a", "c");
    let bc = span_path!("b", "c");

    {
        assert!(root.is_ancestor_of(&a));
        assert!(root.is_ancestor_of(&b));
        assert!(root.is_ancestor_of(&ab));
        assert!(root.is_ancestor_of(&ac));
        assert!(root.is_ancestor_of(&bc));

        assert!(!root.is_ancestor_of(&root));
    }

    {
        assert!(a.is_ancestor_of(&ab));
        assert!(a.is_ancestor_of(&ac));

        assert!(!a.is_ancestor_of(&a));
        assert!(!a.is_ancestor_of(&b));
        assert!(!a.is_ancestor_of(&bc));
    }
}

#[test]
fn common_ancestor() {
    let ref root = span_path!();
    let ref a = span_path!("a");
    let ref ab = span_path!("a", "b");
    let ref b = span_path!("b");
    let ref ac = span_path!("a", "c");

    assert_eq!(root.common_ancestor(&root), None);
    assert_eq!(root.common_ancestor(&a), None);
    assert_eq!(a.common_ancestor(&root), None);
    assert_eq!(root.common_ancestor(&ab), None);
    assert_eq!(ab.common_ancestor(&root), None);

    assert_eq!(a.common_ancestor(&a), Some(root.clone()));
    assert_eq!(a.common_ancestor(&ab), Some(root.clone()));
    assert_eq!(ab.common_ancestor(&a), Some(root.clone()));

    assert_eq!(a.common_ancestor(&b), Some(root.clone()));

    assert_eq!(ab.common_ancestor(&ab), Some(a.clone()));

    assert_eq!(ab.common_ancestor(&ac), Some(a.clone()));
    assert_eq!(ac.common_ancestor(&ab), Some(a.clone()));
}