pub mod model;
pub mod files;

#[cfg(test)]
mod tests {
    use model::VirtualPath;

    use super::*;

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

    // #[test]
    // fn vpath_serde() {
    //     let p1 = serde_json::from_str::<VirtualPath>("\"home/user\"").unwrap();
    //     let p2 = VirtualPath::from("/home/user");
    //     assert_eq!(p1.to_string(), p2.to_string());
    // }
}