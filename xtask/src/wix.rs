//! A check on the Windows installer source (`packaging/windows/vectorcraft.wxs`) that runs with the
//! workspace tests on every platform, so a mistake WiX would only report on a Windows release job
//! is caught in an ordinary PR. WiX refuses two elements of the same kind with the same `Id`
//! (WIX0091 "Duplicate …"), which is what broke the v0.1.1 release's Windows builds.

/// `(kind, Id)` of every element in `wxs` that declares an `Id`, in order.
fn ids(wxs: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = wxs;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        // Skip comments, processing instructions and closing tags.
        if rest.starts_with("!--") {
            rest = rest.find("-->").map_or("", |end| &rest[end + 3..]);
            continue;
        }
        if rest.starts_with('?') || rest.starts_with('/') || rest.starts_with('!') {
            continue;
        }
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        let kind: String = tag.chars().take_while(|c| !c.is_whitespace() && *c != '/').collect();
        if let Some(at) = tag.find(" Id=\"").or_else(|| tag.find("\nId=\"")).or_else(|| tag.find("\tId=\"")) {
            let value = &tag[at + 5..];
            if let Some(close) = value.find('"') {
                out.push((kind, value[..close].to_string()));
            }
        }
        rest = &rest[end..];
    }
    out
}

/// The `kind Id` pairs `wxs` declares more than once.
fn duplicate_ids(wxs: &str) -> Vec<String> {
    let mut seen = std::collections::BTreeMap::<(String, String), usize>::new();
    for k in ids(wxs) {
        *seen.entry(k).or_default() += 1;
    }
    seen.into_iter().filter(|(_, n)| *n > 1).map(|((kind, id), n)| format!("{kind} {id} ×{n}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_installer_declares_each_id_once() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../packaging/windows/vectorcraft.wxs");
        let wxs = std::fs::read_to_string(path).expect("read vectorcraft.wxs");
        assert!(ids(&wxs).len() > 20, "the scan should see the installer's elements");
        assert_eq!(duplicate_ids(&wxs), Vec::<String>::new(), "WiX refuses these (WIX0091): {path}");
    }

    #[test]
    fn a_duplicate_is_caught_and_comments_and_other_kinds_are_not() {
        let twice = r#"<Package><Property Id="A" Value="1" /><!-- <Property Id="A" /> -->
            <Property
              Id="A" Value="2" /><Directory Id="A" /></Package>"#;
        assert_eq!(duplicate_ids(twice), ["Property A ×2"]);
        assert!(duplicate_ids(r#"<Property Id="A" /><Property Id="B" />"#).is_empty());
    }
}
