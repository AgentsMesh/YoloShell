use crate::file_resource_link;
use std::path::Path;

#[test]
fn display_names_and_destination_uris_preserve_separate_markdown_boundaries() {
    assert_eq!(
        file_resource_link("图 [a] (v1)", Path::new("/tmp/a (b)#图%.png")).as_deref(),
        Some(r"[图 \[a\] (v1)](file:///tmp/a%20%28b%29%23%E5%9B%BE%25.png)"),
    );
    assert!(file_resource_link("image", Path::new("relative/image.png")).is_none());
    assert!(file_resource_link("image\nname", Path::new("/tmp/image")).is_none());
    assert!(file_resource_link("image", Path::new("/tmp/unsafe\npath")).is_none());
}
