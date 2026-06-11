import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EmptyState } from "@/components/common/EmptyState";

describe("EmptyState", () => {
  it("renders title, description, and action", () => {
    render(
      <EmptyState
        title="No sources yet"
        description="Drop a folder to start."
        action={<button type="button">Add source</button>}
      />,
    );
    expect(
      screen.getByRole("heading", { name: /no sources yet/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/drop a folder to start/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /add source/i }),
    ).toBeInTheDocument();
  });
});
