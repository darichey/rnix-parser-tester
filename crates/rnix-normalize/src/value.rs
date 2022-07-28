pub enum Anchor {
    Absolute,
    Relative,
    Home,
    Store,
}

pub fn parse_path(s: String) -> (Anchor, String) {
    if let Some(tag) = s.strip_prefix('<') {
        let tag = tag.strip_suffix('>');
        (Anchor::Store, String::from(tag.unwrap()))
    } else if let Some(contents) = s.strip_prefix("~/") {
        (Anchor::Home, String::from(contents))
    } else if s.starts_with('/') {
        (Anchor::Absolute, String::from(s))
    } else {
        (Anchor::Relative, String::from(s))
    }
}
