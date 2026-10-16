//! The skills group as the list shows it: one folded row on a bare `/`, the
//! best few while searching, and the plugins' guides apart under their own
//! fold. A skill is never out of reach — every fold opens with Enter.

use std::collections::BTreeSet;

use crate::rows::{Group, Row, Target};

/// How many skills a search shows before folding the rest.
pub const SHOWN: usize = 4;
/// The shelf word of a plugin's page.
const GUIDE: &str = "guide";
/// The fold a bare `/` shows the skills as.
pub const SKILLS: &str = "skills";
/// The fold the guides wait under once the skills are open.
pub const GUIDES: &str = "guides";
/// The fold past the best few while searching.
pub const MORE: &str = "more";

/// Fold the skills group as `query` and the opened folds say, and name every
/// guide by what it is about.
pub fn fold(groups: &mut [Group], query: &str, expanded: &BTreeSet<String>) {
    if let Some(group) = groups.iter_mut().find(|group| group.name == SKILLS) {
        let rows = std::mem::take(&mut group.rows);
        group.rows = if query.is_empty() {
            bare(rows, expanded)
        } else {
            searched(rows, expanded)
        };
    }
    for row in groups.iter_mut().flat_map(|group| &mut group.rows) {
        if is_guide(row) {
            guide_label(row);
        }
    }
}

/// A bare `/`: one row counting them, then the skills with the guides
/// folded after them, then the guides too.
fn bare(rows: Vec<Row>, expanded: &BTreeSet<String>) -> Vec<Row> {
    let (guides, mut skills): (Vec<Row>, Vec<Row>) = rows.into_iter().partition(is_guide);
    if !expanded.contains(SKILLS) {
        let counts = [count(skills.len(), "skill"), count(guides.len(), GUIDE)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        return (!counts.is_empty())
            .then(|| folded(SKILLS, format!("▸ {}", counts.join(" · "))))
            .into_iter()
            .collect();
    }
    if expanded.contains(GUIDES) {
        skills.extend(guides);
    } else if let Some(word) = count(guides.len(), GUIDE) {
        skills.push(folded(GUIDES, format!("▸ {word}")));
    }
    skills
}

/// A search: the best few, then one row saying how many more match.
fn searched(mut rows: Vec<Row>, expanded: &BTreeSet<String>) -> Vec<Row> {
    if rows.len() <= SHOWN + 1 || expanded.contains(MORE) {
        return rows;
    }
    let more = rows.len() - SHOWN;
    rows.truncate(SHOWN);
    rows.push(folded(MORE, format!("· {more} more skills")));
    rows
}

/// `n` of `noun`, or nothing when there are none.
fn count(n: usize, noun: &str) -> Option<String> {
    match n {
        0 => None,
        1 => Some(format!("1 {noun}")),
        n => Some(format!("{n} {noun}s")),
    }
}

fn folded(fold: &str, label: String) -> Row {
    Row {
        label,
        hint: String::new(),
        alert: None,
        now: String::new(),
        marks: vec![],
        hint_marks: vec![],
        target: Target::Expand(fold.into()),
    }
}

fn is_guide(row: &Row) -> bool {
    matches!(row.target, Target::Skill(_)) && row.now == GUIDE
}

/// `/guide-memory` as `guide › memory`, its marks moved with the letters.
fn guide_label(row: &mut Row) {
    const SHOWN_AS: &str = "guide › ";
    let lead = usize::from(row.label.starts_with('/'));
    let name = &row.label[lead..];
    let topic = name.strip_prefix("guide-");
    // `guide-` is six letters and `guide › ` eight: the topic moves two on.
    let moved = |at: usize| match (topic, at) {
        (None, at) => at + SHOWN_AS.chars().count(),
        (Some(_), 0..=4) => at,
        (Some(_), 5) => 6,
        (Some(_), at) => at + 2,
    };
    row.marks = row
        .marks
        .iter()
        .filter_map(|at| at.checked_sub(lead).map(moved))
        .collect();
    row.label = format!("{SHOWN_AS}{}", topic.unwrap_or(name));
    row.now.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(name: &str, shelf: &str) -> Row {
        Row {
            label: format!("/{name}"),
            hint: format!("what {name} does"),
            alert: None,
            now: shelf.into(),
            marks: vec![],
            hint_marks: vec![],
            target: Target::Skill(name.into()),
        }
    }

    fn group(rows: Vec<Row>) -> Vec<Group> {
        vec![Group {
            name: SKILLS.into(),
            rows,
        }]
    }

    fn labels(groups: &[Group]) -> Vec<String> {
        groups[0].rows.iter().map(|row| row.label.clone()).collect()
    }

    fn opened(folds: &[&str]) -> BTreeSet<String> {
        folds.iter().map(|fold| (*fold).to_owned()).collect()
    }

    #[test]
    fn a_bare_slash_folds_the_skills_into_one_counted_row() {
        let mut groups = group(vec![
            skill("deploy", "project"),
            skill("standup", "yours"),
            skill("guide-memory", GUIDE),
        ]);
        fold(&mut groups, "", &BTreeSet::new());
        assert_eq!(labels(&groups), ["▸ 2 skills · 1 guide"]);
        assert_eq!(groups[0].rows[0].target, Target::Expand(SKILLS.into()));
    }

    #[test]
    fn each_fold_opens_in_place_and_the_guides_wait_under_their_own() {
        let rows = vec![
            skill("deploy", "project"),
            skill("guide-memory", GUIDE),
            skill("guide-mcp", GUIDE),
        ];
        let mut groups = group(rows.clone());
        fold(&mut groups, "", &opened(&[SKILLS]));
        assert_eq!(labels(&groups), ["/deploy", "▸ 2 guides"]);
        let mut groups = group(rows);
        fold(&mut groups, "", &opened(&[SKILLS, GUIDES]));
        assert_eq!(
            labels(&groups),
            ["/deploy", "guide › memory", "guide › mcp"]
        );
        assert_eq!(groups[0].rows[1].now, "", "the label already says guide");
    }

    #[test]
    fn a_search_shows_the_best_four_and_counts_the_rest() {
        let rows = (0..20)
            .map(|n| skill(&format!("s{n}"), "project"))
            .collect::<Vec<_>>();
        let mut groups = group(rows.clone());
        fold(&mut groups, "s", &BTreeSet::new());
        assert_eq!(
            labels(&groups),
            ["/s0", "/s1", "/s2", "/s3", "· 16 more skills"]
        );
        let mut groups = group(rows);
        fold(&mut groups, "s", &opened(&[MORE]));
        assert_eq!(groups[0].rows.len(), 20);
    }

    #[test]
    fn one_more_skill_is_shown_rather_than_counted() {
        let rows = (0..5)
            .map(|n| skill(&format!("s{n}"), "project"))
            .collect::<Vec<_>>();
        let mut groups = group(rows);
        fold(&mut groups, "s", &BTreeSet::new());
        assert_eq!(groups[0].rows.len(), 5);
    }

    #[test]
    fn a_guide_keeps_its_marks_on_the_letters_they_were_on() {
        let mut row = skill("guide-memory", GUIDE);
        // `/guide-memory` matched at g, -, m: 1, 6, 7.
        row.marks = vec![1, 6, 7];
        let mut groups = group(vec![row]);
        fold(&mut groups, "g-m", &BTreeSet::new());
        let row = &groups[0].rows[0];
        assert_eq!(row.label, "guide › memory");
        let marked = row
            .label
            .chars()
            .enumerate()
            .filter(|(at, _)| row.marks.contains(at))
            .map(|(_, c)| c)
            .collect::<String>();
        assert_eq!(marked, "g›m");
    }
}
