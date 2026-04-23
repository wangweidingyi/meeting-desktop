export type SummarySection = {
  title: string;
  items: string[];
};

export type SummaryViewState = {
  version: number;
  isFinal: boolean;
  abstract: string;
  keyPoints: SummarySection;
  decisions: SummarySection;
  risks: SummarySection;
  actionItems: SummarySection;
  lastUpdatedLabel: string;
};
