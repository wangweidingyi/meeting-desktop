export function controlTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/control`;
}

export function controlReplyTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/control/reply`;
}

export function eventsTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/events`;
}

export function sttTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/stt`;
}

export function summaryTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/summary`;
}

export function actionItemsTopic(clientId: string, sessionId: string): string {
  return `meetings/${clientId}/session/${sessionId}/action-items`;
}

