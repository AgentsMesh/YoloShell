use crate::files::{Item, file_urls, shell_word};

#[test]
fn clipboard_file_urls_accept_local_files_in_order_and_decode_paths() {
    let items = file_urls("# comment\r\nfile:///tmp/a%20b.png\r\nfile:///tmp/two\n").unwrap();
    let paths: Vec<_> = items
        .into_iter()
        .map(|i| match i {
            Item::File(p) => p,
            _ => panic!("expected a file"),
        })
        .collect();
    assert_eq!(
        paths,
        [
            std::path::Path::new("/tmp/a b.png"),
            std::path::Path::new("/tmp/two")
        ]
    );
    assert!(file_urls("https://example.com/file").is_err());
    assert!(file_urls("file://another-host/private.png").is_err());
}

#[test]
fn staged_references_are_inert_even_for_shell_metacharacters() {
    assert_eq!(shell_word("/tmp/image.png").unwrap(), "/tmp/image.png");
    assert_eq!(shell_word("/tmp/a 'b;$q").unwrap(), "'/tmp/a '\\''b;$q'");
    assert!(shell_word("relative").is_err());
    assert!(shell_word("/tmp/input\ncommand").is_err());
}

#[test]
fn multiple_staged_files_keep_order_with_distinct_chat_and_shell_syntax() {
    use crate::ReferenceStyle;
    let paths = [
        "/tmp/photo (1).png".to_string(),
        "/tmp/a's #notes.txt".to_string(),
    ];
    let actual = ReferenceStyle::Chat.format(&paths).unwrap();
    let links: Vec<_> = actual.lines().collect();
    assert_eq!(links.len(), paths.len());
    for (link, path) in links.iter().zip(&paths) {
        let uri = link.split_once("](").unwrap().1.strip_suffix(')').unwrap();
        let url = url::Url::parse(uri).unwrap();
        assert_eq!(url.to_file_path().unwrap(), std::path::Path::new(path));
        assert!(!uri.contains([' ', '(', ')', '#']));
    }
    assert_eq!(
        ReferenceStyle::Shell.format(&paths).unwrap(),
        "'/tmp/photo (1).png' '/tmp/a'\\''s #notes.txt'"
    );
}
