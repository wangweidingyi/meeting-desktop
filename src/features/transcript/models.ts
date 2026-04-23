export type TranscriptSegmentView = {
  id: string;
  startMs: number;
  endMs: number;
  text: string;
  isFinal: boolean;
  speakerId: string | null;
  revision: number;
};
