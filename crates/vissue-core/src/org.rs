//! Org 9.8 syntax the tracker has to get right.
//!
//! An `issues.org` is an ordinary Org document. The verbs treat a top-level
//! TODO heading as an issue; everything else in the file is still Org, and a
//! construct Org would not treat as a headline or a property drawer must not
//! become one here.
//!
//! The rules follow the Org 9.8 manual:
//! <https://orgmode.org/manual/>.

use crate::model::TODO_KEYWORDS;

/// Keywords Org writes on the planning line, in the order Org writes them.
pub const PLANNING_KEYS: &[&str] = &["CLOSED", "SCHEDULED", "DEADLINE"];

/// Whether `c` may appear in an Org tag (`[[:alnum:]_@#%]+`).
pub fn is_org_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '@' | '#' | '%')
}

/// A line Org reads as a headline: one or more stars at column 0, then a space.
///
/// Manual 2.1. A line that merely *looks* starred (`**bold**`) is not one.
/// Leading whitespace takes the line off the left margin, so it is not one
/// either.
pub fn is_headline(line: &str) -> bool {
    let stars = line.len() - line.trim_start_matches('*').len();
    stars > 0 && line[stars..].starts_with(' ')
}

/// A level-one headline: `* ` at column 0. That is an issue site.
pub fn is_top_level_headline(line: &str) -> bool {
    line.starts_with("* ")
}

/// `:NAME:` alone on a line, which is how every drawer opens (manual 2.7).
pub fn opens_a_drawer(trimmed: &str) -> bool {
    trimmed.len() > 2
        && trimmed.starts_with(':')
        && trimmed.ends_with(':')
        && !trimmed.eq_ignore_ascii_case(":END:")
        && !trimmed[1..trimmed.len() - 1].contains(char::is_whitespace)
}

/// Nesting of greater blocks (`#+BEGIN_SRC` … `#+END_SRC`) and dynamic
/// blocks (`#+BEGIN: clocktable` … `#+END:`).
///
/// Manual 2.8, 12.6, 16.2. Content inside a block is literal: a line that
/// looks like a headline or a drawer is not one.
#[derive(Debug, Default, Clone)]
pub struct BlockNest {
    depth: usize,
}

impl BlockNest {
    /// Empty nest, at file (or heading) scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a previous line opened a block that has not yet closed.
    pub fn inside(&self) -> bool {
        self.depth > 0
    }

    /// Observe `line`. Returns true when the line is part of a block,
    /// including the `#+BEGIN` / `#+END` lines themselves.
    pub fn observe(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        if is_block_end(trimmed) {
            self.depth = self.depth.saturating_sub(1);
            return true;
        }
        if is_block_begin(trimmed) {
            self.depth += 1;
            return true;
        }
        self.depth > 0
    }
}

fn is_block_begin(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("#+") else {
        return false;
    };
    starts_ignore_ascii(rest, "BEGIN_") || starts_ignore_ascii(rest, "BEGIN:")
}

fn is_block_end(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("#+") else {
        return false;
    };
    starts_ignore_ascii(rest, "END_") || starts_ignore_ascii(rest, "END:")
}

fn starts_ignore_ascii(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// File-local TODO keywords from `#+TODO:` lines, plus the house set.
///
/// Manual 5.2.5. Fast-access keys (`TODO(t)`, `WAIT(w@)`) are stripped.
/// Several `#+TODO:` lines accumulate. The house keywords stay recognised
/// so a preamble that only lists a subset does not drop STARTED headings.
pub fn todo_keywords_from_preamble(preamble: &str) -> Vec<String> {
    todo_keywords_from_lines(&preamble.lines().collect::<Vec<_>>())
}

/// Same as [`todo_keywords_from_preamble`], from already-split lines.
pub fn todo_keywords_from_lines(lines: &[&str]) -> Vec<String> {
    let mut keywords: Vec<String> = TODO_KEYWORDS.iter().map(|s| (*s).to_string()).collect();
    for line in lines {
        let trimmed = line.trim();
        let Some(rest) = strip_file_keyword(trimmed, "TODO") else {
            continue;
        };
        for token in rest.split_whitespace() {
            if token == "|" {
                continue;
            }
            let name = token.split('(').next().unwrap_or(token);
            if name.is_empty() {
                continue;
            }
            if !keywords.iter().any(|k| k == name) {
                keywords.push(name.to_string());
            }
        }
    }
    keywords
}

/// Tags from `#+FILETAGS:` (manual 6 / in-buffer settings).
pub fn filetags_from_preamble(preamble: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in preamble.lines() {
        let Some(rest) = strip_file_keyword(line.trim(), "FILETAGS") else {
            continue;
        };
        for tag in rest.trim().trim_matches(':').split(':') {
            let tag = tag.trim();
            if !tag.is_empty() && tag.chars().all(is_org_tag_char) && !tags.iter().any(|t| t == tag)
            {
                tags.push(tag.to_string());
            }
        }
    }
    tags
}

fn strip_file_keyword<'a>(trimmed: &'a str, name: &str) -> Option<&'a str> {
    let rest = trimmed.strip_prefix("#+")?;
    let (key, value) = rest.split_once(':')?;
    if key.eq_ignore_ascii_case(name) {
        Some(value)
    } else {
        None
    }
}

/// The pieces Org puts on a headline after the stars (manual 2.1, 5.1, 5.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlineBits<'a> {
    /// TODO keyword, when the first word is one of `keywords`.
    pub keyword: Option<&'a str>,
    /// Priority cookie character, when `[#X]` follows the keyword.
    pub priority: Option<char>,
    /// Whether the heading carries the `COMMENT` keyword (manual 13.6).
    pub commented: bool,
    /// Title text, including a trailing tag run and statistics cookies.
    pub rest: &'a str,
}

/// Split the text after `* ` into keyword, priority, COMMENT, and title.
pub fn parse_headline_bits<'a>(after_stars: &'a str, keywords: &[String]) -> HeadlineBits<'a> {
    let trimmed = after_stars.trim();
    let mut rest = trimmed;
    let mut keyword = None;
    if let Some((word, after)) = first_word(rest)
        && is_listed_keyword(word, keywords)
    {
        keyword = Some(word);
        rest = after.trim_start();
    }
    let mut priority = None;
    if let Some((p, after)) = parse_priority_cookie(rest) {
        priority = Some(p);
        rest = after.trim_start();
    }
    let mut commented = false;
    if let Some((word, after)) = first_word(rest)
        && word.eq_ignore_ascii_case("COMMENT")
    {
        commented = true;
        rest = after.trim_start();
    }
    HeadlineBits {
        keyword,
        priority,
        commented,
        rest,
    }
}

fn first_word(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    match s.find(char::is_whitespace) {
        Some(i) => Some((&s[..i], &s[i..])),
        None => Some((s, "")),
    }
}

fn is_listed_keyword(word: &str, keywords: &[String]) -> bool {
    keywords.iter().any(|k| k == word)
}

/// A top-level heading that is an issue: recognised TODO keyword, not COMMENT.
pub fn is_issue_headline(line: &str, keywords: &[String]) -> bool {
    let Some(after) = line.strip_prefix("* ") else {
        return false;
    };
    let bits = parse_headline_bits(after, keywords);
    bits.keyword.is_some() && !bits.commented
}

/// Split a leading `[#A]` cookie off a heading. Any other shape yields `None`.
pub fn parse_priority_cookie(after: &str) -> Option<(char, &str)> {
    let rest = after.strip_prefix("[#")?;
    let mut chars = rest.char_indices();
    let (_, priority) = chars.next()?;
    let (close, bracket) = chars.next()?;
    if bracket != ']' {
        return None;
    }
    Some((priority, &rest[close + 1..]))
}

/// Split trailing statistics cookies (`[2/5]`, `[33%]`) off a title.
///
/// Manual 5.5. Cookies sit after the title and before the tag run.
pub fn split_statistics_cookies(text: &str) -> (String, Option<String>) {
    let mut trimmed = text.trim_end().to_string();
    let mut cookies = Vec::new();
    loop {
        let Some(open) = trimmed.rfind('[') else {
            break;
        };
        if !trimmed.ends_with(']') {
            break;
        }
        let cookie = &trimmed[open..];
        if !is_statistics_cookie(cookie) {
            break;
        }
        let prefix = trimmed[..open].trim_end();
        if open > 0 && !trimmed[..open].ends_with(char::is_whitespace) {
            break;
        }
        cookies.push(cookie.to_string());
        trimmed = prefix.to_string();
    }
    cookies.reverse();
    if cookies.is_empty() {
        (text.trim_end().to_string(), None)
    } else {
        (trimmed, Some(cookies.join(" ")))
    }
}

fn is_statistics_cookie(cookie: &str) -> bool {
    let Some(inner) = cookie.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return false;
    };
    if let Some((a, b)) = inner.split_once('/') {
        return !a.is_empty()
            && !b.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && b.chars().all(|c| c.is_ascii_digit());
    }
    inner
        .strip_suffix('%')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Consume one Org timestamp or timestamp range at the start of `s`.
///
/// Manual 8.1: active `<>`, inactive `[]`, diary sexps `<%%(...)>`,
/// time ranges on one stamp, and ranges of two stamps joined by `--`.
/// Repeaters (`+1w`, `++1w`, `.+1w`) and warnings (`-2d`, `--2d`) live
/// inside the brackets, so the first closing delimiter is enough.
pub fn take_timestamp(s: &str) -> Option<(&str, &str)> {
    let start = s.trim_start();
    let leading = s.len() - start.len();
    if start.starts_with("<%%") {
        let end = start.find('>')?;
        let consumed = leading + end + 1;
        return Some((s[leading..consumed].trim_end(), s[consumed..].trim_start()));
    }
    let close = match start.chars().next()? {
        '<' => '>',
        '[' => ']',
        _ => return None,
    };
    let end = start.find(close)?;
    let mut consumed = leading + end + 1;
    let rest = &s[consumed..];
    if let Some(after) = rest.strip_prefix("--") {
        let after = after.trim_start();
        if after.starts_with('<') || after.starts_with('[') || after.starts_with("<%%") {
            let close2 = if after.starts_with('[') { ']' } else { '>' };
            let end2 = after.find(close2)?;
            consumed = s.len() - after.len() + end2 + 1;
        }
    }
    Some((s[leading..consumed].trim_end(), s[consumed..].trim_start()))
}

/// Read an Org planning line into `KEY -> timestamp` pairs.
///
/// Org packs several onto one line. A line holding anything else is not a
/// planning line at all, so a body sentence that opens on `DEADLINE:` stays
/// body.
pub fn parse_planning_line(line: &str) -> Vec<(String, String)> {
    let mut rest = line.trim();
    let mut found = Vec::new();
    while !rest.is_empty() {
        let Some(key) = PLANNING_KEYS
            .iter()
            .find(|key| rest.starts_with(&format!("{key}:")))
        else {
            return Vec::new();
        };
        let after = rest[key.len() + 1..].trim_start();
        let Some((ts, next)) = take_timestamp(after) else {
            return Vec::new();
        };
        found.push(((*key).to_string(), ts.to_string()));
        rest = next;
    }
    found
}

/// Whether `trimmed` opens a planning line (any planning key and a colon).
pub fn is_planning_line(trimmed: &str) -> bool {
    PLANNING_KEYS
        .iter()
        .any(|key| trimmed.starts_with(key) && trimmed[key.len()..].starts_with(':'))
}

/// Ids named by Org links in `body` that also sit in `known_ids`.
///
/// Manual 4.1 / 4.4: `[[id:foo]]`, `[[id:foo][desc]]`, `<id:foo>`, and a
/// bare `id:foo`. `[[foo]]` and `[[file:x.org::foo]]` still resolve when
/// the target or the search fragment is a known id.
pub fn org_link_targets(body: &str, known_ids: &std::collections::HashSet<&str>) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let raw = &after_start[..end];
        let target = raw.split_once("][").map_or(raw, |(target, _)| target);
        push_link_target(&mut targets, target, known_ids);
        rest = &after_start[end + 2..];
    }
    rest = body;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            break;
        };
        push_link_target(&mut targets, &after[..end], known_ids);
        rest = &after[end + 1..];
    }
    rest = body;
    while let Some(start) = rest.find("id:") {
        let after = &rest[start + 3..];
        let len = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(after.len());
        let id = &after[..len];
        if !id.is_empty() && known_ids.contains(id) && !targets.iter().any(|t| t == id) {
            targets.push(id.to_string());
        }
        rest = &after[len.max(1)..];
    }
    targets
}

fn push_link_target(
    targets: &mut Vec<String>,
    raw: &str,
    known_ids: &std::collections::HashSet<&str>,
) {
    let target = raw.trim();
    let target = target.strip_prefix("id:").unwrap_or(target);
    let target = target.rsplit_once("::").map_or(target, |(_, fragment)| {
        fragment.strip_prefix('#').unwrap_or(fragment)
    });
    let target = target.strip_prefix('#').unwrap_or(target);
    if known_ids.contains(target) && !targets.iter().any(|t| t == target) {
        targets.push(target.to_string());
    }
}

/// Property key as written, with a trailing `+` (append) stripped.
///
/// Manual 7.1: `:var+: value` appends to `:var:`.
pub fn property_key_and_append(key: &str) -> (&str, bool) {
    match key.strip_suffix('+') {
        Some(bare) if !bare.is_empty() => (bare, true),
        _ => (key, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn house() -> Vec<String> {
        TODO_KEYWORDS.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_headline_needs_stars_then_a_space_at_column_zero() {
        assert!(is_headline("* TODO a title"));
        assert!(is_headline("*** deeper"));
        assert!(is_top_level_headline("* TODO a title"));
        assert!(!is_top_level_headline("** child"));
        assert!(!is_headline("**bold** at the start of a line"));
        assert!(!is_headline(" * indented is not a headline"));
        assert!(!is_headline("not a headline"));
    }

    #[test]
    fn greater_and_dynamic_blocks_hide_their_contents() {
        let mut nest = BlockNest::new();
        assert!(!nest.inside());
        assert!(nest.observe("#+BEGIN_SRC org"));
        assert!(nest.inside());
        assert!(nest.observe("* TODO quoted"));
        assert!(nest.observe("#+begin_example"));
        assert!(nest.observe("* still quoted"));
        assert!(nest.observe("#+end_example"));
        assert!(nest.inside());
        assert!(nest.observe("#+END_SRC"));
        assert!(!nest.inside());
        assert!(nest.observe("  #+BEGIN: clocktable :scope file"));
        assert!(nest.observe("* TODO inside clocktable"));
        assert!(nest.observe("  #+END:"));
        assert!(!nest.inside());
    }

    #[test]
    fn file_local_todo_keywords_accumulate_and_keep_the_house_set() {
        let keys = todo_keywords_from_preamble(
            "#+TITLE: x\n#+TODO: TODO(t) WAIT(w@) | DONE(d!)\n#+TODO: HOLD | CANCELLED\n",
        );
        for expected in [
            "TODO",
            "STARTED",
            "BLOCKED",
            "DONE",
            "CANCELLED",
            "WAIT",
            "HOLD",
        ] {
            assert!(
                keys.iter().any(|k| k == expected),
                "{keys:?} missing {expected}"
            );
        }
    }

    #[test]
    fn comment_and_section_headlines_are_not_issues() {
        let keys = house();
        assert!(is_issue_headline("* TODO Ship it", &keys));
        assert!(is_issue_headline("* DONE [#A] Ship it", &keys));
        assert!(!is_issue_headline("* COMMENT Archive", &keys));
        assert!(!is_issue_headline("* TODO COMMENT hidden", &keys));
        assert!(!is_issue_headline("* Notes", &keys));
        assert!(!is_issue_headline("** TODO child", &keys));
    }

    #[test]
    fn a_file_local_keyword_is_an_issue() {
        let keys = todo_keywords_from_preamble("#+TODO: TODO WAIT | DONE\n");
        assert!(is_issue_headline("* WAIT Parked", &keys));
        assert!(!is_issue_headline("* HOLD Parked", &keys));
    }

    #[test]
    fn statistics_cookies_split_off_the_title() {
        assert_eq!(
            split_statistics_cookies("Break it down [2/5]"),
            ("Break it down".into(), Some("[2/5]".into()))
        );
        assert_eq!(
            split_statistics_cookies("Break it down [2/5] [40%]"),
            ("Break it down".into(), Some("[2/5] [40%]".into()))
        );
        assert_eq!(
            split_statistics_cookies("Array [2/3] leftover"),
            ("Array [2/3] leftover".into(), None)
        );
        assert_eq!(
            split_statistics_cookies("Not a cookie [n/a]"),
            ("Not a cookie [n/a]".into(), None)
        );
    }

    #[test]
    fn timestamps_include_ranges_repeaters_and_diary_sexps() {
        let (ts, rest) = take_timestamp("<2026-09-01 Tue +1w -2d> leftover").unwrap();
        assert_eq!(ts, "<2026-09-01 Tue +1w -2d>");
        assert_eq!(rest, "leftover");
        let (ts, rest) = take_timestamp("<2026-09-01 Tue>--<2026-09-08 Tue>").unwrap();
        assert_eq!(ts, "<2026-09-01 Tue>--<2026-09-08 Tue>");
        assert!(rest.is_empty());
        let (ts, _) = take_timestamp("[2026-09-01 Tue 09:00-17:00]").unwrap();
        assert_eq!(ts, "[2026-09-01 Tue 09:00-17:00]");
        let (ts, rest) = take_timestamp("<%%(diary-float t 4 2)> next").unwrap();
        assert_eq!(ts, "<%%(diary-float t 4 2)>");
        assert_eq!(rest, "next");
    }

    #[test]
    fn a_planning_line_keeps_a_range_and_rejects_prose() {
        let found = parse_planning_line(
            "CLOSED: [2026-08-14 Fri 03:33] SCHEDULED: <2026-09-01 Tue>--<2026-09-08 Tue> DEADLINE: <2026-09-15 Mon +1w>",
        );
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(found[1].1, "<2026-09-01 Tue>--<2026-09-08 Tue>");
        assert_eq!(found[2].1, "<2026-09-15 Mon +1w>");
        assert!(parse_planning_line("DEADLINE: is discussed in the design note.").is_empty());
    }

    #[test]
    fn org_links_include_brackets_angles_and_bare_ids() {
        let known: HashSet<&str> = ["atlas-1a2b", "beacon-5j6k"].into_iter().collect();
        let body =
            "See [[id:atlas-1a2b][the parser]] and <id:beacon-5j6k> plus id:atlas-1a2b again.";
        let found = org_link_targets(body, &known);
        assert_eq!(found, vec!["atlas-1a2b", "beacon-5j6k"]);
    }

    #[test]
    fn filetags_parse_the_in_buffer_keyword() {
        assert_eq!(
            filetags_from_preamble("#+FILETAGS: :issues:parser:\n"),
            vec!["issues", "parser"]
        );
    }

    #[test]
    fn property_plus_appends() {
        assert_eq!(property_key_and_append("BLOCKED_BY+"), ("BLOCKED_BY", true));
        assert_eq!(property_key_and_append("ID"), ("ID", false));
    }
}
