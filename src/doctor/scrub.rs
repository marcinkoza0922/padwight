//! Takes what is personal out of a bug report: the user's name in paths, the host name, device
//! addresses and serial numbers, and the other identifiers that tie a report to a person. The
//! report is pasted into a public issue, so this runs over all of it, whichever section it came from.

/// What to look for. Build it with [`Scrubber::new`] and add what the report found with [`Scrubber::secret`].
#[derive(Default)]
pub struct Scrubber {
    home: Option<String>,
    user: Option<String>,
    host: Vec<String>,
    /// Values that are replaced wherever they appear: serial numbers, `uniq` strings, machine ids.
    secrets: Vec<String>,
}

/// Keys whose values are never kept, in the setup files and the config. Compared in lower case.
const REDACTED_KEYS: [&str; 10] =
    ["nickname", "serial", "uniq", "token", "password", "secret", "api_key", "apikey", "webhook", "email"];

impl Scrubber {
    pub fn new(home: Option<&str>, user: Option<&str>, host: Option<&str>) -> Self {
        let home = home.map(|h| h.trim_end_matches('/')).filter(|h| h.len() > 1).map(str::to_owned);
        // A one or two letter user name would match inside every word.
        let user = user.map(str::trim).filter(|u| u.len() >= 3).map(str::to_owned);
        let mut hosts = Vec::new();
        if let Some(host) = host.map(str::trim).filter(|h| h.len() >= 3) {
            hosts.push(host.to_owned());
            if let Some((short, _)) = host.split_once('.').filter(|(s, _)| s.len() >= 3) {
                hosts.push(short.to_owned());
            }
        }
        Scrubber { home, user, host: hosts, secrets: Vec::new() }
    }

    /// Replaces `value`, wherever it appears, with `<serial>`. Values under four characters are ignored.
    pub fn secret(&mut self, value: &str) {
        let value = value.trim();
        if value.len() >= 4 && !self.secrets.iter().any(|s| s == value) {
            self.secrets.push(value.to_owned());
            // Longest first, so a value that holds another is replaced whole.
            self.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        }
    }

    pub fn scrub(&self, text: &str) -> String {
        let mut out = text.to_owned();
        for secret in &self.secrets {
            out = out.replace(secret.as_str(), "<serial>");
        }
        if let Some(home) = &self.home {
            out = replace_path_prefix(&out, home, "~");
        }
        out = other_homes(&out);
        if let Some(user) = &self.user {
            out = replace_word(&out, user, "<user>");
        }
        for host in &self.host {
            out = replace_word(&out, host, "<host>");
        }
        out = replace_tokens(&out, mac, "xx:xx:xx:xx:xx:xx");
        out = replace_tokens(&out, uuid, "<uuid>");
        out = replace_tokens(&out, hex32, "<machine-id>");
        out = replace_tokens(&out, steam_id, "<steam-id>");
        out = replace_tokens(&out, ipv4, "<ip>");
        out = emails(&out);
        out.lines().map(redact_line).collect::<Vec<_>>().join("\n") + if out.ends_with('\n') { "\n" } else { "" }
    }
}

/// `home` followed by `/` or the end of a path, so `/home/ann` doesn't touch `/home/annabel`.
fn replace_path_prefix(text: &str, home: &str, with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let after = &rest[at + home.len()..];
        out.push_str(&rest[..at]);
        if after.chars().next().is_none_or(|c| !is_word(c)) {
            out.push_str(with);
        } else {
            out.push_str(home);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Home folders that aren't this user's: `/home/<name>`, `/var/home/<name>`, `/Users/<name>` and
/// the removable-media mounts that carry a name.
fn other_homes(text: &str) -> String {
    let mut out = text.to_owned();
    for prefix in ["/var/home/", "/home/", "/Users/", "/run/media/", "/media/"] {
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.find(prefix) {
            let (head, tail) = rest.split_at(at + prefix.len());
            result.push_str(head);
            let name_len = tail.find(['/', ' ', '\n', '"', '\'', ':', ')']).unwrap_or(tail.len());
            let name = &tail[..name_len];
            // Already scrubbed names, and the shared `/home/linuxbrew`, stay.
            if name.is_empty() || name.starts_with('<') || name == "linuxbrew" {
                result.push_str(name);
            } else {
                result.push_str("<user>");
            }
            rest = &tail[name_len..];
        }
        result.push_str(rest);
        out = result;
    }
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Replaces `word` where it stands alone (not inside a longer name), ignoring case.
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let lower = text.to_lowercase();
    let word = word.to_lowercase();
    // Lower-casing can change byte lengths for a few scripts; then leave the text alone.
    if lower.len() != text.len() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut from = 0;
    while let Some(found) = lower[from..].find(&word) {
        let start = from + found;
        let end = start + word.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        from = end;
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            continue;
        }
        out.push_str(&text[last..start]);
        out.push_str(with);
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

/// Replaces each token `matcher` recognizes. A token starts at the beginning of a word, and the
/// matcher says how many bytes it takes.
fn replace_tokens(text: &str, matcher: fn(&[u8]) -> Option<usize>, with: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut last = 0;
    while i < bytes.len() {
        let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'.');
        if boundary
            && text.is_char_boundary(i)
            && let Some(n) = matcher(&bytes[i..])
            && bytes.get(i + n).is_none_or(|b| !b.is_ascii_alphanumeric())
        {
            out.push_str(&text[last..i]);
            out.push_str(with);
            i += n;
            last = i;
            continue;
        }
        i += 1;
    }
    out.push_str(&text[last..]);
    out
}

/// `aa:bb:cc:dd:ee:ff` or with dashes: a MAC or Bluetooth address.
fn mac(b: &[u8]) -> Option<usize> {
    if b.len() < 17 {
        return None;
    }
    let sep = b[2];
    if sep != b':' && sep != b'-' {
        return None;
    }
    (0..17)
        .all(|i| if i % 3 == 2 { b[i] == sep } else { b[i].is_ascii_hexdigit() })
        .then_some(17)
}

fn uuid(b: &[u8]) -> Option<usize> {
    if b.len() < 36 {
        return None;
    }
    (0..36)
        .all(|i| if matches!(i, 8 | 13 | 18 | 23) { b[i] == b'-' } else { b[i].is_ascii_hexdigit() })
        .then_some(36)
}

/// 32 hex digits: a machine id.
fn hex32(b: &[u8]) -> Option<usize> {
    (b.len() >= 32 && b[..32].iter().all(u8::is_ascii_hexdigit)).then_some(32)
}

/// A 64-bit Steam id: 17 digits starting 7656119.
fn steam_id(b: &[u8]) -> Option<usize> {
    (b.len() >= 17 && b[..17].iter().all(u8::is_ascii_digit) && b.starts_with(b"7656119")).then_some(17)
}

/// A dotted quad, except loopback and the unspecified address, which say nothing about anyone.
fn ipv4(b: &[u8]) -> Option<usize> {
    let mut pos = 0;
    let mut first = 0u32;
    for octet in 0..4 {
        let digits = b[pos..].iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || digits > 3 {
            return None;
        }
        let value: u32 = std::str::from_utf8(&b[pos..pos + digits]).ok()?.parse().ok()?;
        if value > 255 {
            return None;
        }
        if octet == 0 {
            first = value;
        }
        pos += digits;
        if octet < 3 {
            if b.get(pos) != Some(&b'.') {
                return None;
            }
            pos += 1;
        }
    }
    // Another `.digit` after it makes it a longer version number.
    if b.get(pos) == Some(&b'.') && b.get(pos + 1).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    (first != 127 && first != 0).then_some(pos)
}

/// `name@domain.tld` becomes `<email>`.
fn emails(text: &str) -> String {
    let local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_ascii_alphanumeric() || ".-".contains(c);
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('@') {
        let head = &rest[..at];
        let tail = &rest[at + 1..];
        let start = head.rfind(|c: char| !local(c)).map_or(0, |i| i + head[i..].chars().next().map_or(1, char::len_utf8));
        let end = tail.find(|c: char| !domain(c)).unwrap_or(tail.len());
        let host = tail[..end].trim_end_matches('.');
        if start < head.len() && host.contains('.') && !host.starts_with('.') {
            out.push_str(&head[..start]);
            out.push_str("<email>");
            rest = &tail[host.len()..];
        } else {
            out.push_str(head);
            out.push('@');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// `key = "value"` where the key is on [`REDACTED_KEYS`], or `Key: value` for a serial number or
/// address: the value is replaced. Titles in a rule's quoted text on a log line are replaced too.
fn redact_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    for sep in ['=', ':'] {
        if let Some((key, _)) = trimmed.split_once(sep) {
            let name = key.trim().to_lowercase();
            let name = name.trim_matches(['"', '\'']);
            if REDACTED_KEYS.contains(&name) || (sep == ':' && matches!(name, "serial number" | "uniq" | "serial")) {
                let quote = if sep == '=' { "\"<redacted>\"" } else { "<redacted>" };
                let gap = if sep == '=' { " " } else { "" };
                return format!("{indent}{}{gap}{sep} {quote}", key.trim_end());
            }
        }
    }
    if line.to_lowercase().contains("title") { hide_quoted(line) } else { line.to_owned() }
}

/// Replaces what stands between “ and ” (or between double quotes) with `<title>`.
fn hide_quoted(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        let close = match c {
            '“' => '”',
            '"' => '"',
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push(c);
        let mut inside = String::new();
        let mut closed = false;
        for d in chars.by_ref() {
            if d == close {
                closed = true;
                break;
            }
            inside.push(d);
        }
        out.push_str(if inside.is_empty() { "" } else { "<title>" });
        if closed {
            out.push(close);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scrubber() -> Scrubber {
        let mut s = Scrubber::new(Some("/home/ann"), Some("ann"), Some("annbox.local"));
        s.secret("0123456789AB");
        s
    }

    #[test]
    fn home_becomes_tilde_and_other_homes_a_placeholder() {
        let s = scrubber();
        assert_eq!(s.scrub("/home/ann/.config/padwight/config.toml"), "~/.config/padwight/config.toml");
        assert_eq!(s.scrub("/home/annabel/x and /home/ann"), "/home/<user>/x and ~");
        assert_eq!(s.scrub("/var/home/bob/x /run/media/carl/USB"), "/var/home/<user>/x /run/media/<user>/USB");
        assert_eq!(s.scrub("/home/linuxbrew/.linuxbrew"), "/home/linuxbrew/.linuxbrew");
    }

    #[test]
    fn user_and_host_names_are_replaced_as_words() {
        let s = scrubber();
        assert_eq!(s.scrub("session of Ann on annbox"), "session of <user> on <host>");
        assert_eq!(s.scrub("ANNBOX.local and annbox: ann-pc, channel"), "<host> and <host>: ann-pc, channel");
        assert_eq!(s.scrub("Hannah"), "Hannah");
    }

    #[test]
    fn short_names_are_left_alone() {
        let s = Scrubber::new(Some("/home/al"), Some("al"), Some("pc"));
        assert_eq!(s.scrub("a sale at the pc"), "a sale at the pc");
        assert_eq!(s.scrub("/home/al/x"), "~/x");
    }

    #[test]
    fn device_addresses_and_ids_are_replaced() {
        let s = scrubber();
        assert_eq!(s.scrub("uniq 3c:a9:AB:12:34:56 and 3c-a9-ab-12-34-56."), "uniq xx:xx:xx:xx:xx:xx and xx:xx:xx:xx:xx:xx.");
        assert_eq!(s.scrub("serial 0123456789AB!"), "serial <serial>!");
        assert_eq!(s.scrub("id 123e4567-e89b-12d3-a456-426614174000"), "id <uuid>");
        assert_eq!(s.scrub("machine 0123456789abcdef0123456789abcdef"), "machine <machine-id>");
        assert_eq!(s.scrub("steam 76561198012345678 app 2868840"), "steam <steam-id> app 2868840");
    }

    #[test]
    fn ip_addresses_go_but_versions_and_loopback_stay() {
        let s = scrubber();
        assert_eq!(s.scrub("from 192.168.1.20 and 127.0.0.1"), "from <ip> and 127.0.0.1");
        assert_eq!(s.scrub("kernel 7.3.0.203.5 and 999.1.1.1"), "kernel 7.3.0.203.5 and 999.1.1.1");
    }

    #[test]
    fn vendor_product_ids_are_not_addresses() {
        let s = scrubber();
        assert_eq!(s.scrub("3537:1079 054c:0ce6 Bus 0003"), "3537:1079 054c:0ce6 Bus 0003");
        assert_eq!(s.scrub("HID_ID=0003:0000054C:00000CE6"), "HID_ID=0003:0000054C:00000CE6");
    }

    #[test]
    fn emails_are_replaced() {
        let s = scrubber();
        assert_eq!(s.scrub("mail me at jo.doe+x@example.co.uk."), "mail me at <email>.");
        assert_eq!(s.scrub("see @ home, a@b"), "see @ home, a@b");
    }

    #[test]
    fn redacted_keys_lose_their_values_but_rule_values_stay() {
        let s = scrubber();
        let toml = "name = \"Doom\"\n  nickname = \"Ann's pad\"\nexecutable = \"doom.exe\"\nuniq: abc\n";
        assert_eq!(s.scrub(toml), "name = \"Doom\"\n  nickname = \"<redacted>\"\nexecutable = \"doom.exe\"\nuniq: <redacted>\n");
    }

    #[test]
    fn window_titles_in_log_lines_are_hidden() {
        let s = scrubber();
        assert_eq!(s.scrub("Doom matches Window title “Secret Project - editor”"), "Doom matches Window title “<title>”");
        assert_eq!(s.scrub("brave matches Executable “brave”"), "brave matches Executable “brave”");
    }

    #[test]
    fn keeps_a_missing_final_newline_and_non_ascii_text() {
        let s = scrubber();
        assert_eq!(s.scrub("Größe ✓\nline"), "Größe ✓\nline");
        assert_eq!(s.scrub("Größe ✓\n"), "Größe ✓\n");
    }
}
