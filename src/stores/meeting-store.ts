import { create } from "zustand";

export type MeetingItem = {
  id: string;
  title: string;
  room: string;
  time: string;
  owner: string;
};

type MeetingState = {
  focusMode: boolean;
  meetings: MeetingItem[];
  toggleFocusMode: () => void;
  addQuickMeeting: () => void;
};

const initialMeetings: MeetingItem[] = [
  {
    id: "kickoff-sync",
    title: "产品 Kickoff",
    room: "3F Orbit",
    time: "09:30",
    owner: "Mia",
  },
  {
    id: "weekly-review",
    title: "周会复盘",
    room: "在线会议室 A",
    time: "13:30",
    owner: "Ethan",
  },
  {
    id: "design-critique",
    title: "交互评审",
    room: "5F Aurora",
    time: "16:00",
    owner: "Olivia",
  },
];

export const useMeetingStore = create<MeetingState>((set) => ({
  focusMode: false,
  meetings: initialMeetings,
  toggleFocusMode: () => {
    set((state) => ({ focusMode: !state.focusMode }));
  },
  addQuickMeeting: () => {
    set((state) => ({
      meetings: [
        ...state.meetings,
        {
          id: `quick-${state.meetings.length + 1}`,
          title: `快速站会 ${state.meetings.length - 1}`,
          room: "Lobby Booth",
          time: "18:15",
          owner: "Auto Scheduler",
        },
      ],
    }));
  },
}));
