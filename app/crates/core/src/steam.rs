use std::path::PathBuf;

use crate::vdf;

pub const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;

pub fn steam_id64(account_id: u32) -> String {
    (STEAM_ID64_BASE + u64::from(account_id)).to_string()
}

pub fn most_recent_login(vdf_text: &str) -> Option<String> {
    let users = vdf::parse(vdf_text)?;
    users
        .as_obj()?
        .iter()
        .filter(|(id, _)| id.len() == 17 && id.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(id, entry)| {
            let t = entry.get("Timestamp")?.as_str()?.trim().parse::<i64>().ok()?;
            Some((t, id.clone()))
        })
        .max()
        .map(|(_, id)| id)
}

pub fn login_users_path() -> Option<PathBuf> {
    crate::game::steam_root().map(|root| root.join("config").join("loginusers.vdf"))
}

pub fn active_steam_id() -> Option<String> {
    #[cfg(windows)]
    if let Some(id) = registry_active_user() {
        return Some(id);
    }
    let text = std::fs::read_to_string(login_users_path()?).ok()?;
    most_recent_login(&text)
}

#[cfg(windows)]
fn registry_active_user() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam\ActiveProcess")
        .ok()?;
    let account: u32 = key.get_value("ActiveUser").ok()?;
    (account != 0).then(|| steam_id64(account))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_ids_become_the_steam_id64_wavu_links_to() {
        assert_eq!(steam_id64(123_456_789), "76561198083722517");
    }

    const LOGIN_USERS: &str = r#""users"
{
	"76561198083722517"
	{
		"AccountName"		"account-a"
		"PersonaName"		"persona-a"
		"RememberPassword"		"1"
		"AutoLogin"		"1"
		"Timestamp"		"1789261501"
	}
	"76561198012345678"
	{
		"AccountName"		"account-b"
		"PersonaName"		"persona-b"
		"AutoLogin"		"0"
		"Timestamp"		"1788825582"
	}
}
"#;

    #[test]
    fn the_newest_sign_in_is_the_account_signed_in_now() {
        assert_eq!(most_recent_login(LOGIN_USERS).as_deref(), Some("76561198083722517"));
        let switched = LOGIN_USERS.replace("1788825582", "1789300000");
        assert_eq!(most_recent_login(&switched).as_deref(), Some("76561198012345678"));
    }

    #[test]
    fn a_damaged_file_names_nobody() {
        assert_eq!(most_recent_login(""), None);
        assert_eq!(most_recent_login(r#""users" { "notanid" { "Timestamp" "5" } }"#), None);
    }
}
