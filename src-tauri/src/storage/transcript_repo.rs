use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranscriptSegmentRecord {
    pub segment_id: String,
    pub meeting_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub is_final: bool,
    pub speaker_id: Option<String>,
    pub revision: u64,
}

pub struct TranscriptRepo;

impl TranscriptRepo {
    pub fn upsert(
        connection: &Connection,
        segment: &TranscriptSegmentRecord,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO transcript_segments (
                segment_id,
                meeting_id,
                start_ms,
                end_ms,
                text,
                is_final,
                speaker_id,
                revision
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(segment_id) DO UPDATE SET
                meeting_id = excluded.meeting_id,
                start_ms = excluded.start_ms,
                end_ms = excluded.end_ms,
                text = excluded.text,
                is_final = excluded.is_final,
                speaker_id = excluded.speaker_id,
                revision = excluded.revision
            ",
            params![
                segment.segment_id,
                segment.meeting_id,
                segment.start_ms,
                segment.end_ms,
                segment.text,
                segment.is_final,
                segment.speaker_id,
                segment.revision
            ],
        )?;

        Ok(())
    }

    pub fn find_by_segment_id(
        connection: &Connection,
        segment_id: &str,
    ) -> Result<Option<TranscriptSegmentRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT
                segment_id,
                meeting_id,
                start_ms,
                end_ms,
                text,
                is_final,
                speaker_id,
                revision
            FROM transcript_segments
            WHERE segment_id = ?1
            ",
        )?;

        let mut rows = statement.query([segment_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(TranscriptSegmentRecord {
            segment_id: row.get(0)?,
            meeting_id: row.get(1)?,
            start_ms: row.get(2)?,
            end_ms: row.get(3)?,
            text: row.get(4)?,
            is_final: row.get(5)?,
            speaker_id: row.get(6)?,
            revision: row.get(7)?,
        }))
    }

    pub fn list_by_meeting(
        connection: &Connection,
        meeting_id: &str,
    ) -> Result<Vec<TranscriptSegmentRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT
                segment_id,
                meeting_id,
                start_ms,
                end_ms,
                text,
                is_final,
                speaker_id,
                revision
            FROM transcript_segments
            WHERE meeting_id = ?1
            ORDER BY start_ms ASC, revision ASC
            ",
        )?;

        let rows = statement.query_map([meeting_id], |row| {
            Ok(TranscriptSegmentRecord {
                segment_id: row.get(0)?,
                meeting_id: row.get(1)?,
                start_ms: row.get(2)?,
                end_ms: row.get(3)?,
                text: row.get(4)?,
                is_final: row.get(5)?,
                speaker_id: row.get(6)?,
                revision: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
    }
}

#[cfg(test)]
mod tests {
    use super::{TranscriptRepo, TranscriptSegmentRecord};
    use crate::storage::db::Database;

    #[test]
    fn transcript_repo_updates_segment_revision_and_final_state() {
        let database = Database::open_in_memory().unwrap();

        database
            .with_connection(|connection| {
                TranscriptRepo::upsert(
                    connection,
                    &TranscriptSegmentRecord {
                        segment_id: "segment-1".to_string(),
                        meeting_id: "meeting-1".to_string(),
                        start_ms: 0,
                        end_ms: 1_200,
                        text: "先记录增量版本".to_string(),
                        is_final: false,
                        speaker_id: None,
                        revision: 1,
                    },
                )?;
                TranscriptRepo::upsert(
                    connection,
                    &TranscriptSegmentRecord {
                        segment_id: "segment-1".to_string(),
                        meeting_id: "meeting-1".to_string(),
                        start_ms: 0,
                        end_ms: 1_400,
                        text: "这是最终版本".to_string(),
                        is_final: true,
                        speaker_id: None,
                        revision: 2,
                    },
                )
            })
            .unwrap();

        let segment = database
            .with_connection(|connection| {
                TranscriptRepo::find_by_segment_id(connection, "segment-1")
            })
            .unwrap()
            .unwrap();

        assert_eq!(segment.revision, 2);
        assert!(segment.is_final);
        assert_eq!(segment.text, "这是最终版本");
    }
}
