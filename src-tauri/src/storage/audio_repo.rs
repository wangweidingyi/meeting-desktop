use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAssetRecord {
    pub meeting_id: String,
    pub mic_original_path: Option<String>,
    pub system_original_path: Option<String>,
    pub mixed_uplink_path: Option<String>,
}

pub struct AudioRepo;

impl AudioRepo {
    pub fn upsert(
        connection: &Connection,
        record: &AudioAssetRecord,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO audio_assets (
                meeting_id,
                mic_original_path,
                system_original_path,
                mixed_uplink_path
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(meeting_id) DO UPDATE SET
                mic_original_path = excluded.mic_original_path,
                system_original_path = excluded.system_original_path,
                mixed_uplink_path = excluded.mixed_uplink_path
            ",
            params![
                record.meeting_id,
                record.mic_original_path,
                record.system_original_path,
                record.mixed_uplink_path
            ],
        )?;

        Ok(())
    }

    pub fn find_by_meeting_id(
        connection: &Connection,
        meeting_id: &str,
    ) -> Result<Option<AudioAssetRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT meeting_id, mic_original_path, system_original_path, mixed_uplink_path
            FROM audio_assets
            WHERE meeting_id = ?1
            ",
        )?;

        let mut rows = statement.query([meeting_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(AudioAssetRecord {
            meeting_id: row.get(0)?,
            mic_original_path: row.get(1)?,
            system_original_path: row.get(2)?,
            mixed_uplink_path: row.get(3)?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioAssetRecord, AudioRepo};
    use crate::storage::db::Database;

    #[test]
    fn upsert_and_load_audio_asset_paths() {
        let database = Database::open_in_memory().unwrap();
        let record = AudioAssetRecord {
            meeting_id: "meeting-1".to_string(),
            mic_original_path: Some("audio/meeting-1/mic.wav".to_string()),
            system_original_path: Some("audio/meeting-1/system.wav".to_string()),
            mixed_uplink_path: Some("audio/meeting-1/mixed.wav".to_string()),
        };

        database
            .with_connection(|connection| AudioRepo::upsert(connection, &record))
            .unwrap();

        let loaded = database
            .with_connection(|connection| AudioRepo::find_by_meeting_id(connection, "meeting-1"))
            .unwrap()
            .unwrap();

        assert_eq!(loaded, record);
    }
}
