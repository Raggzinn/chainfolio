import { renderHook } from "@testing-library/react";
import { useIdleTimer } from "./use-idle-lock";

describe("useIdleTimer", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("fires after the idle period", () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(5, onIdle));
    vi.advanceTimersByTime(5 * 60_000 - 1);
    expect(onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(onIdle).toHaveBeenCalledOnce();
  });

  it("restarts the countdown on activity", () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(1, onIdle));
    vi.advanceTimersByTime(50_000);
    window.dispatchEvent(new KeyboardEvent("keydown"));
    vi.advanceTimersByTime(50_000);
    expect(onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(10_000);
    expect(onIdle).toHaveBeenCalledOnce();
  });

  it("does nothing when disabled", () => {
    const onIdle = vi.fn();
    renderHook(() => useIdleTimer(0, onIdle));
    vi.advanceTimersByTime(24 * 60 * 60_000);
    expect(onIdle).not.toHaveBeenCalled();
  });
});
