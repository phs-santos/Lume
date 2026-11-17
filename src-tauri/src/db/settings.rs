use super::DbError;
use rusqlite::Connection;

use rusqlite::OptionalExtension;

pub fn setting_get(c: &Connection, key: &str) -> Result<Option<String>, DbError> {
    Ok(c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0)).optional()?)
}

pub fn setting_set(c: &Connection, key: &str, value: &str) -> Result<(), DbError> {
    c.execute(
        "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_missing_then_set_then_overwrite() {
        let (_d, c) = crate::db::test_conn();
        assert_eq!(setting_get(&c, "theme").unwrap(), None);
        setting_set(&c, "theme", "papel").unwrap();
        setting_set(&c, "theme", "escuro").unwrap();
        assert_eq!(setting_get(&c, "theme").unwrap().as_deref(), Some("escuro"));
    }
}
