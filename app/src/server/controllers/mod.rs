pub mod auth;
pub mod files;
pub mod websocket;

#[cfg(test)]
mod tests {
    use crate::server::models::files::{VirtualPath, VirtualPathError};

    #[test]
    fn vpath_semantics() {
        let p1 = VirtualPath::from("/home/user");
        assert!(!p1.is_dir());
        let p2 = VirtualPath::from("/home/user/");
        assert!(p2.is_dir());
        assert!(p1.path_parts() == p2.path_parts());

        assert_eq!(p1.to_string(), p2.to_string());
        let p3 = VirtualPath::from("/home/user/testthing");
        let p4 = VirtualPath::from("/home/usert/estthing");
        assert!(p3.child_of(&p2));
        assert!(!p4.child_of(&p2));
    }

    #[test]
    fn vpath_ergonomics() {
        // Test Display trait
        let path = VirtualPath::from("root/documents/");
        assert_eq!(format!("{path}"), "root/documents");

        // Test Clone
        let cloned = path.clone();
        assert_eq!(path.to_string(), cloned.to_string());

        // Test AsRef<Path>
        let path_ref: &std::path::Path = path.as_ref();
        assert!(path_ref.to_string_lossy().contains("root"));

        // Test try_from_string with validation
        assert!(VirtualPath::try_from_string("root/valid/path").is_ok());
        assert_eq!(
            VirtualPath::try_from_string("invalid/path"),
            Err(VirtualPathError::InvalidPrefix)
        );
        assert_eq!(
            VirtualPath::try_from_string(""),
            Err(VirtualPathError::EmptyPath)
        );

        // Test path manipulation methods
        let root = VirtualPath::root();
        let docs = root.join("documents").unwrap();
        assert_eq!(docs.to_string(), "root/documents");

        let file = docs.join_file("test.txt").unwrap();
        assert_eq!(file.to_string(), "root/documents/test.txt");
        assert_eq!(file.extension(), Some("txt"));
        assert_eq!(file.file_stem(), Some("test"));

        // Test depth calculation
        assert_eq!(root.depth(), 0);
        assert_eq!(docs.depth(), 1);
        assert_eq!(file.depth(), 2);
    }

    // #[test]
    // fn vpath_serde() {
    //     let p1 = serde_json::from_str::<VirtualPath>("\"home/user\"").unwrap();
    //     let p2 = VirtualPath::from("/home/user");
    //     assert_eq!(p1.to_string(), p2.to_string());
    // }
}
