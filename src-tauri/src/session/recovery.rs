use serde::{Deserialize, Serialize};

use crate::storage::checkpoint_repo::SessionCheckpointRecord;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryPlan {
    pub meeting_id: String,
    pub replay_from_ms: u64,
    pub replay_until_ms: u64,
    pub pending_duration_ms: u64,
}

pub fn plan_recovery(
    checkpoint: &SessionCheckpointRecord,
    local_mixed_duration_ms: u64,
) -> Option<RecoveryPlan> {
    if local_mixed_duration_ms <= checkpoint.last_uploaded_mixed_ms {
        return None;
    }

    Some(RecoveryPlan {
        meeting_id: checkpoint.meeting_id.clone(),
        replay_from_ms: checkpoint.last_uploaded_mixed_ms,
        replay_until_ms: local_mixed_duration_ms,
        pending_duration_ms: local_mixed_duration_ms - checkpoint.last_uploaded_mixed_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::plan_recovery;
    use crate::storage::checkpoint_repo::SessionCheckpointRecord;

    #[test]
    fn recovery_plan_uses_checkpoint_and_local_audio_duration() {
        let checkpoint = SessionCheckpointRecord {
            meeting_id: "meeting-1".to_string(),
            last_control_seq: 0,
            last_udp_seq_sent: 900,
            last_uploaded_mixed_ms: 180_000,
            last_transcript_segment_revision: 0,
            last_summary_version: 0,
            last_action_item_version: 0,
            local_recording_state: "recording".to_string(),
            recovery_token: Some("recover-1".to_string()),
            updated_at: "2026-04-22T10:00:00Z".to_string(),
        };

        let plan = plan_recovery(&checkpoint, 240_000).unwrap();

        assert_eq!(plan.replay_from_ms, 180_000);
        assert_eq!(plan.replay_until_ms, 240_000);
        assert_eq!(plan.pending_duration_ms, 60_000);
    }
}
