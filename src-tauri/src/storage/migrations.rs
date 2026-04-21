use rusqlite::Connection;

pub fn run(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meetings (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            duration_ms INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS transcript_segments (
            segment_id TEXT PRIMARY KEY,
            meeting_id TEXT NOT NULL,
            start_ms INTEGER NOT NULL,
            end_ms INTEGER NOT NULL,
            text TEXT NOT NULL,
            is_final INTEGER NOT NULL DEFAULT 0,
            speaker_id TEXT,
            revision INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS summary_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            meeting_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            updated_at TEXT NOT NULL,
            abstract TEXT NOT NULL,
            key_points TEXT NOT NULL,
            decisions TEXT NOT NULL,
            risks TEXT NOT NULL,
            action_items TEXT NOT NULL,
            is_final INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS action_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            meeting_id TEXT NOT NULL,
            content TEXT NOT NULL,
            is_final INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS audio_assets (
            meeting_id TEXT PRIMARY KEY,
            mic_original_path TEXT,
            system_original_path TEXT,
            mixed_uplink_path TEXT
        );

        CREATE TABLE IF NOT EXISTS session_checkpoints (
            meeting_id TEXT PRIMARY KEY,
            last_control_seq INTEGER NOT NULL DEFAULT 0,
            last_udp_seq_sent INTEGER NOT NULL DEFAULT 0,
            last_uploaded_mixed_ms INTEGER NOT NULL DEFAULT 0,
            last_transcript_segment_revision INTEGER NOT NULL DEFAULT 0,
            last_summary_version INTEGER NOT NULL DEFAULT 0,
            last_action_item_version INTEGER NOT NULL DEFAULT 0,
            local_recording_state TEXT NOT NULL DEFAULT 'idle',
            recovery_token TEXT,
            updated_at TEXT NOT NULL
        );
        ",
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::run;

    #[test]
    fn migrations_create_required_tables() {
        let connection = Connection::open_in_memory().unwrap();
        run(&connection).unwrap();

        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .unwrap();
        let tables = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"meetings".to_string()));
        assert!(tables.contains(&"session_checkpoints".to_string()));
    }
}
