import { useGSAP } from "@gsap/react";
import gsap from "gsap";

gsap.registerPlugin(useGSAP);
gsap.defaults({ ease: "power3.out", duration: 0.6 });

export const MOTION_OK = "(prefers-reduced-motion: no-preference)";

export function prefersReducedMotion() {
  return typeof window !== "undefined" && !window.matchMedia(MOTION_OK).matches;
}

export { gsap, useGSAP };
