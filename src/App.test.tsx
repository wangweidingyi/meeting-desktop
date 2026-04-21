import { render, screen } from "@testing-library/react";

import App from "@/App";

describe("App", () => {
  it("renders meeting workspace navigation", () => {
    render(<App />);

    expect(screen.getByRole("link", { name: /会议主页/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /会中工作台/i })).toBeInTheDocument();
  });
});
