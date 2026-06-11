import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "@/App";

describe("App shell", () => {
  it("renders the three primary surfaces and status bar", () => {
    render(<App />);
    expect(
      screen.getByRole("navigation", { name: /workspaces/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("complementary", { name: /sources/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/ask your sources/i)).toBeInTheDocument();
    expect(screen.getByText(/local-only/i)).toBeInTheDocument();
  });

  it("shows preview mode in the status bar outside Tauri", () => {
    render(<App />);
    expect(screen.getByText(/preview mode/i)).toBeInTheDocument();
  });
});
