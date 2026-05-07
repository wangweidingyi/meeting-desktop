use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use reqwest::blocking::{Client, RequestBuilder};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionItemsRecord {
    pub meeting_id: String,
    pub version: u64,
    pub updated_at: String,
    pub items: Vec<String>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioAssetRecord {
    pub meeting_id: String,
    pub mic_original_path: Option<String>,
    pub system_original_path: Option<String>,
    pub mixed_uplink_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncedMeetingRecord {
    pub id: String,
    pub client_id: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: u64,
}

pub trait MeetingSync: Send + Sync {
    fn set_auth_token(&self, token: Option<String>);
    fn upsert_meeting(&self, meeting: &SyncedMeetingRecord) -> Result<(), String>;
    fn upsert_transcript_segment(&self, segment: &TranscriptSegmentRecord) -> Result<(), String>;
    fn upsert_summary_snapshot(&self, summary: &SummarySnapshotRecord) -> Result<(), String>;
    fn apply_action_items(&self, action_items: &ActionItemsRecord) -> Result<(), String>;
    fn upsert_checkpoint(
        &self,
        checkpoint: &SessionCheckpointRecord,
    ) -> Result<SessionCheckpointRecord, String>;
    fn find_checkpoint(&self, meeting_id: &str) -> Result<Option<SessionCheckpointRecord>, String>;
    fn upsert_audio_assets(&self, assets: &AudioAssetRecord) -> Result<(), String>;
    fn find_audio_assets(&self, meeting_id: &str) -> Result<Option<AudioAssetRecord>, String>;
}

#[derive(Default)]
struct InMemoryState {
    meetings: HashMap<String, SyncedMeetingRecord>,
    segments: HashMap<String, HashMap<String, TranscriptSegmentRecord>>,
    summaries: HashMap<String, SummarySnapshotRecord>,
    checkpoints: HashMap<String, SessionCheckpointRecord>,
    audio_assets: HashMap<String, AudioAssetRecord>,
    auth_token: Option<String>,
}

#[derive(Default)]
pub struct InMemoryMeetingSync {
    state: Mutex<InMemoryState>,
}

impl InMemoryMeetingSync {
    pub fn transcript_segments(&self, meeting_id: &str) -> Vec<TranscriptSegmentRecord> {
        let state = self.state.lock().unwrap();
        let mut items = state
            .segments
            .get(meeting_id)
            .map(|items| items.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        items.sort_by_key(|segment| {
            (
                segment.start_ms,
                segment.revision,
                segment.segment_id.clone(),
            )
        });
        items
    }

    pub fn latest_summary(&self, meeting_id: &str) -> Option<SummarySnapshotRecord> {
        self.state
            .lock()
            .unwrap()
            .summaries
            .get(meeting_id)
            .cloned()
    }
}

impl MeetingSync for InMemoryMeetingSync {
    fn set_auth_token(&self, token: Option<String>) {
        self.state.lock().unwrap().auth_token = token;
    }

    fn upsert_meeting(&self, meeting: &SyncedMeetingRecord) -> Result<(), String> {
        self.state
            .lock()
            .unwrap()
            .meetings
            .insert(meeting.id.clone(), meeting.clone());
        Ok(())
    }

    fn upsert_transcript_segment(&self, segment: &TranscriptSegmentRecord) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        state
            .segments
            .entry(segment.meeting_id.clone())
            .or_default()
            .insert(segment.segment_id.clone(), segment.clone());
        Ok(())
    }

    fn upsert_summary_snapshot(&self, summary: &SummarySnapshotRecord) -> Result<(), String> {
        self.state
            .lock()
            .unwrap()
            .summaries
            .insert(summary.meeting_id.clone(), summary.clone());
        Ok(())
    }

    fn apply_action_items(&self, action_items: &ActionItemsRecord) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let mut summary = state
            .summaries
            .get(&action_items.meeting_id)
            .cloned()
            .unwrap_or(SummarySnapshotRecord {
                meeting_id: action_items.meeting_id.clone(),
                version: action_items.version,
                updated_at: action_items.updated_at.clone(),
                abstract_text: String::new(),
                key_points: vec![],
                decisions: vec![],
                risks: vec![],
                action_items: vec![],
                is_final: action_items.is_final,
            });
        summary.version = summary.version.max(action_items.version);
        summary.updated_at = action_items.updated_at.clone();
        summary.action_items = action_items.items.clone();
        summary.is_final = summary.is_final || action_items.is_final;
        state
            .summaries
            .insert(action_items.meeting_id.clone(), summary);
        Ok(())
    }

    fn upsert_checkpoint(
        &self,
        checkpoint: &SessionCheckpointRecord,
    ) -> Result<SessionCheckpointRecord, String> {
        self.state
            .lock()
            .unwrap()
            .checkpoints
            .insert(checkpoint.meeting_id.clone(), checkpoint.clone());
        Ok(checkpoint.clone())
    }

    fn find_checkpoint(&self, meeting_id: &str) -> Result<Option<SessionCheckpointRecord>, String> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .checkpoints
            .get(meeting_id)
            .cloned())
    }

    fn upsert_audio_assets(&self, assets: &AudioAssetRecord) -> Result<(), String> {
        self.state
            .lock()
            .unwrap()
            .audio_assets
            .insert(assets.meeting_id.clone(), assets.clone());
        Ok(())
    }

    fn find_audio_assets(&self, meeting_id: &str) -> Result<Option<AudioAssetRecord>, String> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .audio_assets
            .get(meeting_id)
            .cloned())
    }
}

pub struct HttpMeetingSync {
    client: Client,
    base_url: String,
    auth_token: Mutex<Option<String>>,
}

impl HttpMeetingSync {
    pub fn new(base_url: String) -> Result<Self, String> {
        let client = Client::builder()
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            base_url,
            auth_token: Mutex::new(None),
        })
    }

    fn authorized(&self, builder: RequestBuilder) -> Result<RequestBuilder, String> {
        let token = self
            .auth_token
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "desktop backend auth token is not set".to_string())?;
        Ok(builder.bearer_auth(token))
    }

    fn endpoint(&self, suffix: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), suffix)
    }

    fn ensure_success<T>(&self, response: reqwest::blocking::Response) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        if response.status().is_success() {
            response.json::<T>().map_err(|error| error.to_string())
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(format!("backend sync failed ({status}): {body}"))
        }
    }

    fn ensure_empty_success(&self, response: reqwest::blocking::Response) -> Result<(), String> {
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            Err(format!("backend sync failed ({status}): {body}"))
        }
    }
}

impl MeetingSync for HttpMeetingSync {
    fn set_auth_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.auth_token.lock() {
            *guard = token;
        }
    }

    fn upsert_meeting(&self, meeting: &SyncedMeetingRecord) -> Result<(), String> {
        let response = self
            .authorized(
                self.client
                    .put(self.endpoint(&format!("/api/app/meetings/{}", meeting.id)))
                    .json(meeting),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_empty_success(response)
    }

    fn upsert_transcript_segment(&self, segment: &TranscriptSegmentRecord) -> Result<(), String> {
        let response = self
            .authorized(
                self.client
                    .put(self.endpoint(&format!(
                        "/api/app/meetings/{}/transcript-segments/{}",
                        segment.meeting_id, segment.segment_id
                    )))
                    .json(segment),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_empty_success(response)
    }

    fn upsert_summary_snapshot(&self, summary: &SummarySnapshotRecord) -> Result<(), String> {
        let response = self
            .authorized(
                self.client
                    .put(
                        self.endpoint(&format!("/api/app/meetings/{}/summary", summary.meeting_id)),
                    )
                    .json(summary),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_empty_success(response)
    }

    fn apply_action_items(&self, action_items: &ActionItemsRecord) -> Result<(), String> {
        let response = self
            .authorized(
                self.client
                    .put(self.endpoint(&format!(
                        "/api/app/meetings/{}/action-items",
                        action_items.meeting_id
                    )))
                    .json(action_items),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_empty_success(response)
    }

    fn upsert_checkpoint(
        &self,
        checkpoint: &SessionCheckpointRecord,
    ) -> Result<SessionCheckpointRecord, String> {
        let response = self
            .authorized(
                self.client
                    .put(self.endpoint(&format!(
                        "/api/app/meetings/{}/checkpoint",
                        checkpoint.meeting_id
                    )))
                    .json(checkpoint),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_success(response)
    }

    fn find_checkpoint(&self, meeting_id: &str) -> Result<Option<SessionCheckpointRecord>, String> {
        let response = self
            .authorized(
                self.client
                    .get(self.endpoint(&format!("/api/app/meetings/{meeting_id}/checkpoint"))),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.ensure_success(response).map(Some)
    }

    fn upsert_audio_assets(&self, assets: &AudioAssetRecord) -> Result<(), String> {
        let response = self
            .authorized(
                self.client
                    .put(self.endpoint(&format!(
                        "/api/app/meetings/{}/audio-assets",
                        assets.meeting_id
                    )))
                    .json(assets),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        self.ensure_empty_success(response)
    }

    fn find_audio_assets(&self, meeting_id: &str) -> Result<Option<AudioAssetRecord>, String> {
        let response = self
            .authorized(
                self.client
                    .get(self.endpoint(&format!("/api/app/meetings/{meeting_id}/audio-assets"))),
            )?
            .send()
            .map_err(|error| error.to_string())?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.ensure_success(response).map(Some)
    }
}

pub type SharedMeetingSync = Arc<dyn MeetingSync>;
