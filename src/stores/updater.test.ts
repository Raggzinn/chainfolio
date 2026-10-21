const check = vi.fn();
const relaunch = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({ check: () => check() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: () => relaunch() }));
vi.mock("@/lib/platform", () => ({ IS_TAURI: true, IS_MAC: false }));

const { useUpdater } = await import("./updater");

const reset = () =>
  useUpdater.setState({
    status: "idle",
    update: null,
    progress: null,
    error: null,
    lastCheckedAt: null,
    dismissed: false,
  });

describe("updater store", () => {
  beforeEach(() => {
    reset();
    check.mockReset();
    relaunch.mockReset();
  });

  it("reports when the app is up to date", async () => {
    check.mockResolvedValue(null);
    await useUpdater.getState().check();
    expect(useUpdater.getState().status).toBe("upToDate");
  });

  it("exposes an available update", async () => {
    check.mockResolvedValue({ version: "1.2.0" });
    await useUpdater.getState().check();
    expect(useUpdater.getState().status).toBe("available");
    expect(useUpdater.getState().update?.version).toBe("1.2.0");
  });

  it("stays quiet when a background check fails", async () => {
    check.mockRejectedValue(new Error("offline"));
    await useUpdater.getState().check({ silent: true });
    expect(useUpdater.getState().status).toBe("idle");
  });

  it("reports a failed manual check", async () => {
    check.mockRejectedValue(new Error("offline"));
    await useUpdater.getState().check();
    expect(useUpdater.getState()).toMatchObject({ status: "error", error: "offline" });
  });

  it("tracks download progress and relaunches", async () => {
    const downloadAndInstall = vi.fn(async (onEvent: (e: unknown) => void) => {
      onEvent({ event: "Started", data: { contentLength: 100 } });
      onEvent({ event: "Progress", data: { chunkLength: 40 } });
      expect(useUpdater.getState().progress).toBeCloseTo(0.4);
      onEvent({ event: "Progress", data: { chunkLength: 60 } });
      onEvent({ event: "Finished" });
    });
    check.mockResolvedValue({ version: "1.2.0", downloadAndInstall });
    await useUpdater.getState().check();
    await useUpdater.getState().install();
    expect(downloadAndInstall).toHaveBeenCalled();
    expect(useUpdater.getState().status).toBe("ready");
    expect(relaunch).toHaveBeenCalled();
  });
});
