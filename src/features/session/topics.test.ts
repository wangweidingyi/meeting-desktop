import {
  actionItemsTopic,
  controlReplyTopic,
  controlTopic,
  eventsTopic,
  sttTopic,
  summaryTopic,
} from "@/features/session/topics";

describe("meeting session topics", () => {
  it("builds the expected control topic", () => {
    expect(controlTopic("client-a", "session-1")).toBe(
      "meetings/client-a/session/session-1/control",
    );
  });

  it("builds the expected downstream event topics", () => {
    expect(controlReplyTopic("client-a", "session-1")).toBe(
      "meetings/client-a/session/session-1/control/reply",
    );
    expect(eventsTopic("client-a", "session-1")).toBe(
      "meetings/client-a/session/session-1/events",
    );
    expect(sttTopic("client-a", "session-1")).toBe("meetings/client-a/session/session-1/stt");
    expect(summaryTopic("client-a", "session-1")).toBe(
      "meetings/client-a/session/session-1/summary",
    );
    expect(actionItemsTopic("client-a", "session-1")).toBe(
      "meetings/client-a/session/session-1/action-items",
    );
  });
});

