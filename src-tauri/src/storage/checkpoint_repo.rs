use rusqlite::{params, Connection};

use crate::transport::audio_transport::AudioUploadProgress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCheckpointRecord {
    pub meeting_id: String,
    pub last_control_seq: u64,
    pub last_udp_seq_sent: u64,
    pub last_uploaded_mixed_ms: u64,
    pub last_transcript_segment_revision: u64,
    pub last_summary_version: u64,
    pub last_action_item_version: u64,
    pub local_recording_state: String,
    pub recovery_token: Option<String>,
    pub updated_at: String,
}

pub struct CheckpointRepo;

impl CheckpointRepo {
    pub fn upsert(
        connection: &Connection,
        record: &SessionCheckpointRecord,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO session_checkpoints (
                meeting_id,
                last_control_seq,
                last_udp_seq_sent,
                last_uploaded_mixed_ms,
                last_transcript_segment_revision,
                last_summary_version,
                last_action_item_version,
                local_recording_state,
                recovery_token,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(meeting_id) DO UPDATE SET
                last_control_seq = excluded.last_control_seq,
                last_udp_seq_sent = excluded.last_udp_seq_sent,
                last_uploaded_mixed_ms = excluded.last_uploaded_mixed_ms,
                last_transcript_segment_revision = excluded.last_transcript_segment_revision,
                last_summary_version = excluded.last_summary_version,
                last_action_item_version = excluded.last_action_item_version,
                local_recording_state = excluded.local_recording_state,
                recovery_token = excluded.recovery_token,
                updated_at = excluded.updated_at
            ",
            params![
                record.meeting_id,
                record.last_control_seq,
                record.last_udp_seq_sent,
                record.last_uploaded_mixed_ms,
                record.last_transcript_segment_revision,
                record.last_summary_version,
                record.last_action_item_version,
                record.local_recording_state,
                record.recovery_token,
                record.updated_at
            ],
        )?;

        Ok(())
    }

    pub fn find_by_meeting_id(
        connection: &Connection,
        meeting_id: &str,
    ) -> Result<Option<SessionCheckpointRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT
                meeting_id,
                last_control_seq,
                last_udp_seq_sent,
                last_uploaded_mixed_ms,
                last_transcript_segment_revision,
                last_summary_version,
                last_action_item_version,
                local_recording_state,
                recovery_token,
                updated_at
            FROM session_checkpoints
            WHERE meeting_id = ?1
            ",
        )?;

        let mut rows = statement.query([meeting_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(SessionCheckpointRecord {
            meeting_id: row.get(0)?,
            last_control_seq: row.get(1)?,
            last_udp_seq_sent: row.get(2)?,
            last_uploaded_mixed_ms: row.get(3)?,
            last_transcript_segment_revision: row.get(4)?,
            last_summary_version: row.get(5)?,
            last_action_item_version: row.get(6)?,
            local_recording_state: row.get(7)?,
            recovery_token: row.get(8)?,
            updated_at: row.get(9)?,
        }))
    }

    pub fn record_audio_upload(
        connection: &Connection,
        meeting_id: &str,
        progress: &AudioUploadProgress,
        updated_at: &str,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO session_checkpoints (
                meeting_id,
                last_udp_seq_sent,
                last_uploaded_mixed_ms,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(meeting_id) DO UPDATE SET
                last_udp_seq_sent = excluded.last_udp_seq_sent,
                last_uploaded_mixed_ms = excluded.last_uploaded_mixed_ms,
                updated_at = excluded.updated_at
            ",
            params![
                meeting_id,
                progress.sequence,
                progress.last_uploaded_mixed_ms,
                updated_at
            ],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::checkpoint_repo::{CheckpointRepo, SessionCheckpointRecord};
    use crate::storage::db::Database;
    use crate::transport::audio_transport::AudioUploadProgress;

    #[test]
    fn record_audio_upload_updates_mixed_offset_and_sequence() {
        let database = Database::open_in_memory().unwrap();

        database
            .with_connection(|connection| {
                CheckpointRepo::upsert(
                    connection,
                    &SessionCheckpointRecord {
                        meeting_id: "meeting-1".to_string(),
                        last_control_seq: 0,
                        last_udp_seq_sent: 0,
                        last_uploaded_mixed_ms: 0,
                        last_transcript_segment_revision: 0,
                        last_summary_version: 0,
                        last_action_item_version: 0,
                        local_recording_state: "recording".to_string(),
                        recovery_token: None,
                        updated_at: "1000".to_string(),
                    },
                )?;

                CheckpointRepo::record_audio_upload(
                    connection,
                    "meeting-1",
                    &AudioUploadProgress {
                        sequence: 7,
                        last_uploaded_mixed_ms: 1_200,
                    },
                    "1200",
                )
            })
            .unwrap();

        let checkpoint = database
            .with_connection(|connection| {
                CheckpointRepo::find_by_meeting_id(connection, "meeting-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.last_udp_seq_sent, 7);
        assert_eq!(checkpoint.last_uploaded_mixed_ms, 1_200);
        assert_eq!(checkpoint.updated_at, "1200");
    }
}
