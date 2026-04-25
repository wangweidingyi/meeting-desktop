import { invoke } from "@tauri-apps/api/core";

export type DesktopAuthUser = {
  id: string;
  username: string;
  displayName: string;
  role: string;
  status: string;
};

export type DesktopAuthSession = {
  token: string;
  expiresAt?: string;
  user: DesktopAuthUser;
};

type LoginResponse = {
  token: string;
  expires_at?: string;
  user: {
    id: string;
    username: string;
    display_name: string;
    role: string;
    status: string;
  };
};

type RuntimeBackendInfo = {
  controlClientId: string;
  adminApiBaseUrl: string | null;
};

const storageKey = "meeting.desktop.auth";

export function getDesktopAuthSession(): DesktopAuthSession | null {
  const raw = window.localStorage.getItem(storageKey);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as DesktopAuthSession;
  } catch {
    window.localStorage.removeItem(storageKey);
    return null;
  }
}

export function setDesktopAuthSession(session: DesktopAuthSession) {
  window.localStorage.setItem(storageKey, JSON.stringify(session));
}

export function clearDesktopAuthSession() {
  window.localStorage.removeItem(storageKey);
}

export async function loginDesktop(username: string, password: string): Promise<DesktopAuthSession> {
  const runtime = await invoke<RuntimeBackendInfo>("get_runtime_backend_info");
  if (!runtime.adminApiBaseUrl) {
    throw new Error("未配置管理后台地址，无法登录");
  }

  const response = await fetch(`${runtime.adminApiBaseUrl}/api/auth/login`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      username,
      password,
      client_type: "desktop",
      device_id: runtime.controlClientId,
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `登录失败 (${response.status})`);
  }

  const payload = (await response.json()) as LoginResponse;
  const session: DesktopAuthSession = {
    token: payload.token,
    expiresAt: payload.expires_at,
    user: {
      id: payload.user.id,
      username: payload.user.username,
      displayName: payload.user.display_name,
      role: payload.user.role,
      status: payload.user.status,
    },
  };
  setDesktopAuthSession(session);
  return session;
}

export async function logoutDesktop() {
  const runtime = await invoke<RuntimeBackendInfo>("get_runtime_backend_info");
  const session = getDesktopAuthSession();
  if (runtime.adminApiBaseUrl && session?.token) {
    await fetch(`${runtime.adminApiBaseUrl}/api/auth/logout`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${session.token}`,
      },
    });
  }
  clearDesktopAuthSession();
}
