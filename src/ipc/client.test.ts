import { describe, expect, it, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

import { api } from "@/ipc/client";

describe("ipc client", () => {
  beforeEach(() => invokeMock.mockReset());

  it("ping forwards the name argument and returns the pong", async () => {
    invokeMock.mockResolvedValue({ message: "pong", echoed: "x" });
    const pong = await api.ping("x");
    expect(invokeMock).toHaveBeenCalledWith("ping", { name: "x" });
    expect(pong.message).toBe("pong");
  });

  it("startTick passes count and camelCased interval", async () => {
    invokeMock.mockResolvedValue(undefined);
    await api.startTick(3, 200);
    expect(invokeMock).toHaveBeenCalledWith("start_tick", {
      count: 3,
      intervalMs: 200,
    });
  });

  it("appVersion invokes the app_version command", async () => {
    invokeMock.mockResolvedValue("0.0.0");
    expect(await api.appVersion()).toBe("0.0.0");
    expect(invokeMock).toHaveBeenCalledWith("app_version", undefined);
  });
});
