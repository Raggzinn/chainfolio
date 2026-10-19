export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type Side = "right" | "bottom" | "left" | "top";

const GAP = 16;
const MARGIN = 12;

const clamp = (value: number, min: number, max: number) =>
  Math.min(Math.max(value, min), Math.max(min, max));

/**
 * Places a tooltip next to the highlighted target, preferring the side with
 * the most room, and keeps it inside the viewport.
 */
export function placeTooltip(
  target: Rect,
  tooltip: { width: number; height: number },
  viewport: { width: number; height: number },
): { x: number; y: number; side: Side } {
  const room: Record<Side, number> = {
    right: viewport.width - (target.x + target.width) - GAP,
    left: target.x - GAP,
    bottom: viewport.height - (target.y + target.height) - GAP,
    top: target.y - GAP,
  };
  const fits: Record<Side, boolean> = {
    right: room.right >= tooltip.width,
    left: room.left >= tooltip.width,
    bottom: room.bottom >= tooltip.height,
    top: room.top >= tooltip.height,
  };
  const order: Side[] = ["right", "bottom", "left", "top"];
  const side =
    order.find((s) => fits[s]) ?? order.reduce((best, s) => (room[s] > room[best] ? s : best));

  const centerX = target.x + target.width / 2 - tooltip.width / 2;
  const centerY = target.y + target.height / 2 - tooltip.height / 2;
  const maxX = viewport.width - tooltip.width - MARGIN;
  const maxY = viewport.height - tooltip.height - MARGIN;

  switch (side) {
    case "right":
      return { side, x: target.x + target.width + GAP, y: clamp(centerY, MARGIN, maxY) };
    case "left":
      return { side, x: target.x - GAP - tooltip.width, y: clamp(centerY, MARGIN, maxY) };
    case "bottom":
      return { side, x: clamp(centerX, MARGIN, maxX), y: target.y + target.height + GAP };
    case "top":
      return { side, x: clamp(centerX, MARGIN, maxX), y: target.y - GAP - tooltip.height };
  }
}
