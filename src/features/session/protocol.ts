export type ControlMessageType =
  | "session/hello"
  | "session/resume"
  | "session/close"
  | "recording/start"
  | "recording/pause"
  | "recording/resume"
  | "recording/stop"
  | "heartbeat"
  | "ack"
  | "error";

export type ServerEventType =
  | "recording_started"
  | "recording_paused"
  | "recording_resumed"
  | "recording_stopped"
  | "stt_delta"
  | "stt_final"
  | "summary_delta"
  | "summary_final"
  | "action_item_delta"
  | "action_item_final"
  | "heartbeat"
  | "error";

export type AudioFormat = {
  encoding: "pcm_s16le";
  sampleRate: 16000;
  channels: 1;
};

export type MessageEnvelope<TType extends string, TPayload> = {
  version: "v1";
  messageId: string;
  correlationId?: string;
  clientId: string;
  sessionId: string;
  seq: number;
  sentAt: string;
  type: TType;
  payload: TPayload;
};

export type SessionHelloPayload = {
  audio: AudioFormat;
  transport: {
    control: "mqtt";
    audio: "udp";
  };
  features: {
    realtimeTranscript: boolean;
    realtimeSummary: boolean;
    actionItems: boolean;
  };
};

export type SessionHelloMessage = MessageEnvelope<"session/hello", SessionHelloPayload>;

