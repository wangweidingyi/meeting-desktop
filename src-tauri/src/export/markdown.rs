use crate::storage::summary_repo::SummarySnapshotRecord;
use crate::storage::transcript_repo::TranscriptSegmentRecord;

#[derive(Debug, Clone)]
pub struct MeetingMarkdownExport {
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub transcript_segments: Vec<TranscriptSegmentRecord>,
    pub summary: Option<SummarySnapshotRecord>,
}

pub fn export_meeting_markdown(export: &MeetingMarkdownExport) -> String {
    let mut lines = vec![
        format!("# {}", export.title),
        String::new(),
        format!("开始时间：{}", export.started_at),
        format!(
            "结束时间：{}",
            export
                .ended_at
                .clone()
                .unwrap_or_else(|| "未结束".to_string())
        ),
        String::new(),
    ];

    if let Some(summary) = &export.summary {
        lines.extend([
            "## 最终会议纪要".to_string(),
            String::new(),
            summary.abstract_text.clone(),
            String::new(),
            "### 关键要点".to_string(),
        ]);
        lines.extend(summary.key_points.iter().map(|item| format!("- {item}")));
        lines.extend([String::new(), "### 决策".to_string()]);
        lines.extend(summary.decisions.iter().map(|item| format!("- {item}")));
        lines.extend([String::new(), "### 风险".to_string()]);
        lines.extend(summary.risks.iter().map(|item| format!("- {item}")));
        lines.extend([String::new(), "## 行动项".to_string()]);
        lines.extend(summary.action_items.iter().map(|item| format!("- {item}")));
        lines.push(String::new());
    }

    lines.extend(["## 完整逐段转写".to_string(), String::new()]);
    lines.extend(export.transcript_segments.iter().map(|segment| {
        format!(
            "- [{} - {}] {}",
            segment.start_ms, segment.end_ms, segment.text
        )
    }));
    lines.push(String::new());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{export_meeting_markdown, MeetingMarkdownExport};
    use crate::storage::summary_repo::SummarySnapshotRecord;
    use crate::storage::transcript_repo::TranscriptSegmentRecord;

    #[test]
    fn markdown_export_contains_summary_transcript_and_action_items() {
        let markdown = export_meeting_markdown(&MeetingMarkdownExport {
            title: "产品策略例会".to_string(),
            started_at: "2026-04-21 09:30".to_string(),
            ended_at: Some("2026-04-21 10:18".to_string()),
            transcript_segments: vec![TranscriptSegmentRecord {
                segment_id: "segment-1".to_string(),
                meeting_id: "meeting-1".to_string(),
                start_ms: 0,
                end_ms: 1_200,
                text: "主持人：先确认音频链路。".to_string(),
                is_final: true,
                speaker_id: None,
                revision: 2,
            }],
            summary: Some(SummarySnapshotRecord {
                meeting_id: "meeting-1".to_string(),
                version: 3,
                updated_at: "2026-04-21 10:18".to_string(),
                abstract_text: "会议明确 Rust 主控与 mixed 单流方案。".to_string(),
                key_points: vec!["控制链路使用 MQTT".to_string()],
                decisions: vec!["音频链路使用 UDP".to_string()],
                risks: vec!["需要补断线恢复".to_string()],
                action_items: vec!["完善导出能力".to_string()],
                is_final: true,
            }),
        });

        assert!(markdown.contains("# 产品策略例会"));
        assert!(markdown.contains("## 最终会议纪要"));
        assert!(markdown.contains("## 完整逐段转写"));
        assert!(markdown.contains("## 行动项"));
        assert!(markdown.contains("主持人：先确认音频链路。"));
    }
}
