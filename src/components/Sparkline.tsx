import { useRef } from "react";
import type { Sample } from "../state/ThroughputTracker";

const WIDTH = 120;
const HEIGHT = 30;
const SLOTS = 60;
/** Below this a quiet line stays quiet instead of being blown up to the full height. */
const FLOOR = 4 * 1024;

/** A smooth path through the points (Catmull-Rom turned into Béziers). */
function curve(points: [number, number][]): string {
  if (points.length < 2) return "";
  let path = `M${points[0][0]},${points[0][1]}`;
  for (let i = 0; i < points.length - 1; i++) {
    const [p0, p1, p2, p3] = [points[i - 1] ?? points[i], points[i], points[i + 1], points[i + 2] ?? points[i + 1]];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    path += ` C${c1[0].toFixed(1)},${c1[1].toFixed(1)} ${c2[0].toFixed(1)},${c2[1].toFixed(1)} ${p2[0]},${p2[1]}`;
  }
  return path;
}

/** The last minute of traffic. It scrolls one slot to the left every second. */
export function Sparkline({ samples, live }: { samples: readonly Sample[]; live: boolean }) {
  // A new key restarts the slide animation each time a sample arrives.
  const ticks = useRef({ samples, count: 0 });
  if (ticks.current.samples !== samples) ticks.current = { samples, count: ticks.current.count + 1 };

  const step = WIDTH / (SLOTS - 1);
  const totals = samples.map((s) => s.up + s.down);
  const peak = Math.max(FLOOR, ...totals);
  const offset = SLOTS - totals.length;
  const points: [number, number][] = totals.map((total, i) => [
    +((offset + i) * step).toFixed(1),
    +(HEIGHT - 2 - (total / peak) * (HEIGHT - 5)).toFixed(1),
  ]);
  const line = curve(points);
  const area = points.length > 1 ? `${line} L${points[points.length - 1][0]},${HEIGHT} L${points[0][0]},${HEIGHT} Z` : "";

  return (
    <svg className={live ? "spark is-live" : "spark"} viewBox={`0 0 ${WIDTH} ${HEIGHT}`} preserveAspectRatio="none" aria-hidden="true">
      <g key={ticks.current.count} className="spark__slide" style={{ ["--step" as string]: `${step}px` }}>
        <path className="spark__area" d={area} />
        <path className="spark__line" d={line} vectorEffect="non-scaling-stroke" />
      </g>
    </svg>
  );
}
