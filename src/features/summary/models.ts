export type SummarySection = {
  title: string;
  items: string[];
};

export type SummaryViewState = {
  abstract: string;
  keyPoints: SummarySection;
  decisions: SummarySection;
  risks: SummarySection;
  actionItems: SummarySection;
  lastUpdatedLabel: string;
};

