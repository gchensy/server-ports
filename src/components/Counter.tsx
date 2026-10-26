import { motion, useSpring, useTransform } from "motion/react";
import { useEffect } from "react";

/** A number that rolls to its new value instead of jumping. */
export function Counter({ value, format }: { value: number; format: (value: number) => string }) {
  const spring = useSpring(value, { stiffness: 120, damping: 24, mass: 0.7 });
  useEffect(() => spring.set(value), [spring, value]);
  const text = useTransform(spring, (current) => format(Math.round(current)));
  return <motion.span>{text}</motion.span>;
}
