import { render, screen } from "@testing-library/react";

import App from "@/App";

describe("App", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders the login gate before entering the workspace", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: /账号登录/i })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /会议主页/i })).not.toBeInTheDocument();
  });
});
