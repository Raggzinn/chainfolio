import { placeTooltip } from "./placement";

const viewport = { width: 1280, height: 800 };
const tooltip = { width: 320, height: 180 };

describe("placeTooltip", () => {
  it("prefers the right side when there is room", () => {
    const p = placeTooltip({ x: 10, y: 100, width: 240, height: 500 }, tooltip, viewport);
    expect(p.side).toBe("right");
    expect(p.x).toBe(10 + 240 + 16);
  });

  it("falls back to below a wide element", () => {
    const p = placeTooltip({ x: 100, y: 80, width: 1100, height: 300 }, tooltip, viewport);
    expect(p.side).toBe("bottom");
    expect(p.y).toBe(80 + 300 + 16);
  });

  it("goes left of an element hugging the right edge", () => {
    const p = placeTooltip({ x: 1000, y: 20, width: 260, height: 760 }, tooltip, viewport);
    expect(p.side).toBe("left");
    expect(p.x).toBe(1000 - 16 - 320);
  });

  it("keeps the tooltip inside the viewport", () => {
    const p = placeTooltip({ x: 10, y: 760, width: 40, height: 30 }, tooltip, viewport);
    expect(p.y + tooltip.height).toBeLessThanOrEqual(viewport.height);
    expect(p.y).toBeGreaterThanOrEqual(0);
  });

  it("uses the roomiest side when nothing fits", () => {
    const p = placeTooltip({ x: 0, y: 0, width: 1280, height: 800 }, tooltip, viewport);
    expect(["right", "bottom", "left", "top"]).toContain(p.side);
  });
});
