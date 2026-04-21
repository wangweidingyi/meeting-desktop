use rusqlite::{params, Connection};

use crate::session::models::{MeetingRecord, SessionStatus};

pub struct MeetingsRepo;

impl MeetingsRepo {
    pub fn insert(connection: &Connection, meeting: &MeetingRecord) -> Result<(), rusqlite::Error> {
        Self::upsert(connection, meeting)
    }

    pub fn upsert(connection: &Connection, meeting: &MeetingRecord) -> Result<(), rusqlite::Error> {
        connection.execute(
            "
            INSERT INTO meetings (id, title, status, started_at, ended_at, duration_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                status = excluded.status,
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                duration_ms = excluded.duration_ms
            ",
            params![
                meeting.id,
                meeting.title,
                meeting.status.as_db_value(),
                meeting.started_at,
                meeting.ended_at,
                meeting.duration_ms
            ],
        )?;

        Ok(())
    }

    pub fn list_all(connection: &Connection) -> Result<Vec<MeetingRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT id, title, status, started_at, ended_at, duration_ms
            FROM meetings
            ORDER BY started_at DESC
            ",
        )?;

        let meetings = statement
            .query_map([], parse_meeting_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(meetings)
    }

    pub fn list_recoverable(
        connection: &Connection,
    ) -> Result<Vec<MeetingRecord>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "
            SELECT id, title, status, started_at, ended_at, duration_ms
            FROM meetings
            WHERE status IN ('connecting', 'ready', 'recording', 'paused', 'stopping', 'error')
            ORDER BY started_at DESC
            ",
        )?;

        let meetings = statement
            .query_map([], parse_meeting_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(meetings)
    }
}

fn parse_meeting_row(row: &rusqlite::Row<'_>) -> Result<MeetingRecord, rusqlite::Error> {
    Ok(MeetingRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        status: SessionStatus::from_db_value(&row.get::<_, String>(2)?),
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        duration_ms: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::session::models::{MeetingRecord, SessionStatus};
    use crate::storage::db::Database;

    use super::MeetingsRepo;

    #[test]
    fn list_recoverable_returns_incomplete_meetings() {
        let database = Database::open_in_memory().unwrap();

        let mut recoverable = MeetingRecord::new("Recoverable".to_string());
        recoverable.status = SessionStatus::Recording;
        let completed = MeetingRecord {
            status: SessionStatus::Completed,
            ..MeetingRecord::new("Completed".to_string())
        };

        database
            .with_connection(|connection| {
                MeetingsRepo::insert(connection, &recoverable)?;
                MeetingsRepo::insert(connection, &completed)
            })
            .unwrap();

        let meetings = database
            .with_connection(MeetingsRepo::list_recoverable)
            .unwrap();

        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].title, "Recoverable");
    }
}
