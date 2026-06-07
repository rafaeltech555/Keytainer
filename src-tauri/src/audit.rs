use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::vault::{Vault, VaultItem};

/// Passwords scoring below this zxcvbn score (0..=4) are flagged weak.
/// Matches the frontend meter's master-password cutoff (MIN_MASTER_SCORE).
pub const WEAK_SCORE: u8 = 2;

/// A reference to an item in a finding. Never carries the password value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditItemRef {
    pub id: Uuid,
    pub site_name: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReuseGroup {
    pub items: Vec<AuditItemRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WeakItem {
    pub item: AuditItemRef,
    pub score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub reused: Vec<ReuseGroup>,
    pub weak: Vec<WeakItem>,
}

fn item_ref(i: &VaultItem) -> AuditItemRef {
    AuditItemRef {
        id: i.id,
        site_name: i.site_name.clone(),
        username: i.username.clone(),
    }
}

/// zxcvbn score (0..=4) for a password.
fn strength_score(password: &str) -> u8 {
    // zxcvbn v3: `zxcvbn(pw, &[])` returns `Entropy`; `.score()` is a
    // fieldless `Score` enum castable to u8. (If the resolved crate version
    // differs — e.g. 2.x returns `Result<Entropy>` and `.score()` is already
    // u8 — adapt only this one helper.)
    zxcvbn::zxcvbn(password, &[]).score() as u8
}

pub fn audit(vault: &Vault) -> AuditReport {
    // Reuse: group by exact password, skipping empty passwords. Items keep
    // vault order within a group; groups are sorted by the first item's name.
    let mut groups: HashMap<&str, Vec<AuditItemRef>> = HashMap::new();
    for it in &vault.items {
        if it.password.is_empty() {
            continue;
        }
        groups.entry(it.password.as_str()).or_default().push(item_ref(it));
    }
    let mut reused: Vec<ReuseGroup> = groups
        .into_values()
        .filter(|items| items.len() >= 2)
        .map(|items| ReuseGroup { items })
        .collect();
    reused.sort_by(|a, b| {
        a.items[0]
            .site_name
            .to_lowercase()
            .cmp(&b.items[0].site_name.to_lowercase())
    });

    // Weak: score each non-empty password; flag those below WEAK_SCORE.
    let mut weak: Vec<WeakItem> = vault
        .items
        .iter()
        .filter(|i| !i.password.is_empty())
        .filter_map(|i| {
            let score = strength_score(&i.password);
            (score < WEAK_SCORE).then(|| WeakItem {
                item: item_ref(i),
                score,
            })
        })
        .collect();
    weak.sort_by(|a, b| {
        a.item
            .site_name
            .to_lowercase()
            .cmp(&b.item.site_name.to_lowercase())
    });

    AuditReport { reused, weak }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;

    fn item(name: &str, user: &str, pw: &str) -> VaultItem {
        VaultItem {
            id: Uuid::new_v4(),
            site_name: name.into(),
            username: user.into(),
            password: pw.into(),
            totp: None,
            url: None,
            notes: None,
            tags: vec![],
            created_at: 0,
            updated_at: 0,
        }
    }

    fn vault(items: Vec<VaultItem>) -> Vault {
        let mut v = Vault::default();
        v.items = items;
        v
    }

    #[test]
    fn groups_items_sharing_one_password() {
        let v = vault(vec![
            item("GitHub", "a", "hunter2sameshared"),
            item("GitLab", "b", "hunter2sameshared"),
            item("Netflix", "c", "hunter2sameshared"),
        ]);
        let report = audit(&v);
        assert_eq!(report.reused.len(), 1);
        assert_eq!(report.reused[0].items.len(), 3);
    }

    #[test]
    fn distinct_shared_passwords_make_separate_groups() {
        let v = vault(vec![
            item("A", "a", "sharedAAAA1111"),
            item("B", "b", "sharedAAAA1111"),
            item("C", "c", "sharedBBBB2222"),
            item("D", "d", "sharedBBBB2222"),
            item("E", "e", "uniqueEEEE3333"),
        ]);
        let report = audit(&v);
        assert_eq!(report.reused.len(), 2);
    }

    #[test]
    fn flags_weak_but_not_strong_passwords() {
        let v = vault(vec![
            item("Weak", "a", "password"),
            item("Strong", "b", "correct-horse-battery-staple-9173"),
        ]);
        let report = audit(&v);
        let weak_sites: Vec<&str> = report.weak.iter().map(|w| w.item.site_name.as_str()).collect();
        assert!(weak_sites.contains(&"Weak"));
        assert!(!weak_sites.contains(&"Strong"));
    }

    #[test]
    fn skips_empty_passwords_entirely() {
        let v = vault(vec![
            item("NoPass1", "a", ""),
            item("NoPass2", "b", ""),
        ]);
        let report = audit(&v);
        assert!(report.reused.is_empty());
        assert!(report.weak.is_empty());
    }

    #[test]
    fn output_is_sorted_by_site_name() {
        let v = vault(vec![
            item("Zebra", "a", "password"),
            item("Apple", "b", "password"),
        ]);
        let report = audit(&v);
        assert_eq!(report.weak[0].item.site_name, "Apple");
        assert_eq!(report.weak[1].item.site_name, "Zebra");
    }
}
