import { useEffect, useState } from "react";

import type { MeetingListItem } from "@/features/meetings/models";
import { listMeetingHistory } from "@/lib/api/commands";

type MeetingHistoryState = {
  meetings: MeetingListItem[];
  isLoading: boolean;
  error: string | null;
};

export function useMeetingHistory() {
  const [state, setState] = useState<MeetingHistoryState>({
    meetings: [],
    isLoading: true,
    error: null,
  });

  useEffect(() => {
    let disposed = false;

    setState((current) => ({
      meetings: current.meetings,
      isLoading: true,
      error: null,
    }));

    void listMeetingHistory()
      .then((meetings) => {
        if (!disposed) {
          setState({
            meetings,
            isLoading: false,
            error: null,
          });
        }
      })
      .catch((reason) => {
        if (!disposed) {
          setState({
            meetings: [],
            isLoading: false,
            error: String(reason),
          });
        }
      });

    return () => {
      disposed = true;
    };
  }, []);

  return state;
}
