import { render, screen } from "@testing-library/react";
import { MASK } from "@/lib/format";
import { AnimatedNumber } from "./animated-number";

const format = (n: number) => `$${n.toFixed(2)}`;
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe("AnimatedNumber", () => {
  it("masks the value when hidden", () => {
    render(<AnimatedNumber value={1234} format={format} hidden />);
    expect(screen.getByText(MASK)).toBeInTheDocument();
  });

  it("stays masked when hidden again while the count-up is still running", async () => {
    const { container, rerender } = render(<AnimatedNumber value={1234} format={format} hidden />);
    rerender(<AnimatedNumber value={1234} format={format} hidden={false} />);
    await wait(50);
    rerender(<AnimatedNumber value={1234} format={format} hidden />);
    await wait(300);
    expect(container.textContent).toBe(MASK);
  });

  it("shows the final value once revealed", async () => {
    const { container } = render(<AnimatedNumber value={42} format={format} duration={0.05} />);
    await wait(300);
    expect(container.textContent).toBe("$42.00");
  });
});
