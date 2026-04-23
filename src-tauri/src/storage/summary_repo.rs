use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SummarySnapshotRecord {
    pub meeting_id: String,
    pub version: u64,
    pub updated_at: String,
    pub abstract_text: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub risks: Vec<String>,
    pub action_items: Vec<String>,
    pub is_final: bool,
}

pub struct SummaryRepo;

impl SummaryRepo {
    pub fn upsert_snapshot(
        connection: &Connection,
        snapshot: &SummarySnapshotRecord,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO summary_snapshots (
                meeting_id,
                version,
                updated_at,
                abstract,
                key_points,
                decisions,
                risks,
                action_items,
                is_final
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO NOTHING
            ",
            params![
                snapshot.meeting_id,
                snapshot.version,
                snapshot.updated_at,
                snapshot.abstract_text,
                to_json_array(&snapshot.key_points),
                to_json_array(&snapshot.decisions),
                to_json_array(&snapshot.risks),
                to_json_array(&snapshot.action_items),
                snapshot.is_final
            ],
        )?;

        connection.execute(
            "DELETE FROM action_items WHERE meeting_id = ?1",
            [&snapshot.meeting_id],
        )?;

        for action_item in &snapshot.action_items {
            connection.execute(
                "
                INSERT INTO action_items (meeting_id, content, is_final)
                VALUES (?1, ?2, ?3)
                ",
                params![snapshot.meeting_id, action_item, snapshot.is_final],
            )?;
        }

        Ok(())
    }

    pub fn latest_snapshot(
        connection: &Connection,
        meeting_id: &str,
    ) -> Result<Option<SummarySnapshotRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT
                meeting_id,
                version,
                updated_at,
                abstract,
                key_points,
                decisions,
                risks,
                action_items,
                is_final
            FROM summary_snapshots
            WHERE meeting_id = ?1
            ORDER BY version DESC
            LIMIT 1
            ",
        )?;

        let mut rows = statement.query([meeting_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(SummarySnapshotRecord {
            meeting_id: row.get(0)?,
            version: row.get(1)?,
            updated_at: row.get(2)?,
            abstract_text: row.get(3)?,
            key_points: from_json_array(row.get::<_, String>(4)?),
            decisions: from_json_array(row.get::<_, String>(5)?),
            risks: from_json_array(row.get::<_, String>(6)?),
            action_items: from_json_array(row.get::<_, String>(7)?),
            is_final: row.get(8)?,
        }))
    }

    pub fn list_action_items(
        connection: &Connection,
        meeting_id: &str,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT content
            FROM action_items
            WHERE meeting_id = ?1
            ORDER BY id ASC
            ",
        )?;

        let items = statement
            .query_map([meeting_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }
}

fn to_json_array(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

fn from_json_array(items: String) -> Vec<String> {
    serde_json::from_str(&items).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{SummaryRepo, SummarySnapshotRecord};
    use crate::storage::db::Database;

    #[test]
    fn summary_repo_stores_delta_versions_and_final_snapshot() {
        let database = Database::open_in_memory().unwrap();

        database
            .with_connection(|connection| {
                SummaryRepo::upsert_snapshot(
                    connection,
                    &SummarySnapshotRecord {
                        meeting_id: "meeting-1".to_string(),
                        version: 1,
                        updated_at: "1000".to_string(),
                        abstract_text: "先保存增量摘要".to_string(),
                        key_points: vec!["音频链路先跑通".to_string()],
                        decisions: vec![],
                        risks: vec!["需要处理断线恢复".to_string()],
                        action_items: vec!["继续完善协议".to_string()],
                        is_final: false,
                    },
                )?;
                SummaryRepo::upsert_snapshot(
                    connection,
                    &SummarySnapshotRecord {
                        meeting_id: "meeting-1".to_string(),
                        version: 3,
                        updated_at: "3000".to_string(),
                        abstract_text: "保存最终纪要".to_string(),
                        key_points: vec!["Rust 主控".to_string()],
                        decisions: vec!["首版使用 MQTT + UDP".to_string()],
                        risks: vec![],
                        action_items: vec!["联调服务端协议".to_string()],
                        is_final: true,
                    },
                )
            })
            .unwrap();

        let snapshot = database
            .with_connection(|connection| SummaryRepo::latest_snapshot(connection, "meeting-1"))
            .unwrap()
            .unwrap();
        let action_items = database
            .with_connection(|connection| SummaryRepo::list_action_items(connection, "meeting-1"))
            .unwrap();

        assert!(snapshot.is_final);
        assert_eq!(snapshot.version, 3);
        assert_eq!(snapshot.abstract_text, "保存最终纪要");
        assert_eq!(action_items, vec!["联调服务端协议".to_string()]);
    }
}
