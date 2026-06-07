use uuid::Uuid;

use super::{Vault, VaultItem};
use crate::error::{AppError, AppResult};

/// Current unix seconds. Wrapped so tests can override later if needed.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn add_item(vault: &mut Vault, mut item: VaultItem) -> Uuid {
    if item.id.is_nil() {
        item.id = Uuid::new_v4();
    }
    let ts = now_unix();
    if item.created_at == 0 {
        item.created_at = ts;
    }
    item.updated_at = ts;

    register_tags(vault, &item.tags);
    let id = item.id;
    vault.items.push(item);
    id
}

pub fn update_item(vault: &mut Vault, updated: VaultItem) -> AppResult<()> {
    let id = updated.id;
    let pos = vault
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or(AppError::ItemNotFound(id))?;

    let created_at = vault.items[pos].created_at;
    let mut next = updated;
    next.created_at = created_at;
    next.updated_at = now_unix();
    register_tags(vault, &next.tags);
    vault.items[pos] = next;
    Ok(())
}

pub fn delete_item(vault: &mut Vault, id: Uuid) -> AppResult<()> {
    let pos = vault
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or(AppError::ItemNotFound(id))?;
    vault.items.remove(pos);
    Ok(())
}

pub fn search<'a>(vault: &'a Vault, query: &str) -> Vec<&'a VaultItem> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vault.items.iter().collect();
    }
    vault
        .items
        .iter()
        .filter(|i| {
            i.site_name.to_lowercase().contains(&q)
                || i.username.to_lowercase().contains(&q)
                || i.url
                    .as_ref()
                    .map(|u| u.to_lowercase().contains(&q))
                    .unwrap_or(false)
                || i.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect()
}

fn register_tags(vault: &mut Vault, tags: &[String]) {
    for t in tags {
        let t = t.trim();
        if !t.is_empty() && !vault.tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            vault.tags.push(t.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{TotpEntry, VaultItem};

    fn item(name: &str, user: &str) -> VaultItem {
        VaultItem {
            id: Uuid::nil(),
            site_name: name.into(),
            username: user.into(),
            password: "pw".into(),
            totp: None,
            url: None,
            notes: None,
            tags: vec![],
            password_history: vec![],
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn add_assigns_id_and_timestamps() {
        let mut v = Vault::default();
        let id = add_item(&mut v, item("GitHub", "alice"));
        assert_eq!(v.items.len(), 1);
        assert!(!id.is_nil());
        assert!(v.items[0].created_at > 0);
        assert_eq!(v.items[0].updated_at, v.items[0].created_at);
    }

    #[test]
    fn update_preserves_created_at_and_bumps_updated_at() {
        let mut v = Vault::default();
        let id = add_item(&mut v, item("GitHub", "alice"));
        let created = v.items[0].created_at;
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let mut new_item = v.items[0].clone();
        new_item.password = "newpw".into();
        update_item(&mut v, new_item).unwrap();

        assert_eq!(v.items[0].created_at, created);
        assert!(v.items[0].updated_at > created);
        assert_eq!(v.items[0].password, "newpw");
        assert_eq!(v.items[0].id, id);
    }

    #[test]
    fn update_missing_id_errors() {
        let mut v = Vault::default();
        let mut ghost = item("ghost", "x");
        ghost.id = Uuid::new_v4();
        let err = update_item(&mut v, ghost).unwrap_err();
        assert!(matches!(err, AppError::ItemNotFound(_)));
    }

    #[test]
    fn delete_removes_only_target() {
        let mut v = Vault::default();
        let a = add_item(&mut v, item("a", "a"));
        let b = add_item(&mut v, item("b", "b"));
        delete_item(&mut v, a).unwrap();
        assert_eq!(v.items.len(), 1);
        assert_eq!(v.items[0].id, b);
    }

    #[test]
    fn search_matches_name_user_url_tag_case_insensitively() {
        let mut v = Vault::default();
        let mut gh = item("GitHub", "alice");
        gh.url = Some("https://github.com".into());
        gh.tags = vec!["work".into()];
        add_item(&mut v, gh);

        let mut gl = item("GitLab", "bob");
        gl.tags = vec!["personal".into()];
        add_item(&mut v, gl);

        assert_eq!(search(&v, "git").len(), 2);
        assert_eq!(search(&v, "ALICE").len(), 1);
        assert_eq!(search(&v, "github.com").len(), 1);
        assert_eq!(search(&v, "personal").len(), 1);
        assert_eq!(search(&v, "").len(), 2);
    }

    #[test]
    fn registers_new_tags_case_insensitive_dedup() {
        let mut v = Vault::default();
        let mut i = item("a", "a");
        i.tags = vec!["Work".into(), "Personal".into()];
        add_item(&mut v, i);

        let mut j = item("b", "b");
        j.tags = vec!["work".into(), "Finance".into()];
        add_item(&mut v, j);

        assert_eq!(v.tags.len(), 3);
        let lower: Vec<String> = v.tags.iter().map(|s| s.to_lowercase()).collect();
        assert!(lower.contains(&"work".to_string()));
        assert!(lower.contains(&"personal".to_string()));
        assert!(lower.contains(&"finance".to_string()));
    }

    #[test]
    fn totp_entry_round_trips_through_serde() {
        let mut v = Vault::default();
        let mut i = item("a", "a");
        i.totp = Some(TotpEntry::new_default("JBSWY3DPEHPK3PXP"));
        add_item(&mut v, i);

        let json = serde_json::to_string(&v).unwrap();
        let v2: Vault = serde_json::from_str(&json).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn item_without_history_field_deserializes_to_empty() {
        let json = r#"{"id":"00000000-0000-0000-0000-000000000000","site_name":"S","username":"u","password":"p","tags":[],"created_at":0,"updated_at":0}"#;
        let item: VaultItem = serde_json::from_str(json).unwrap();
        assert!(item.password_history.is_empty());
    }
}
