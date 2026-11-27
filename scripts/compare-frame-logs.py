#!/usr/bin/env python3
"""Compare VIctory's interval-average frame logs (not individual presents)."""
import argparse
import math
import re
import statistics
from pathlib import Path

FRAME = re.compile(
    r"([\d.]+) fps, ([\d.]+) ms/frame \(([\d.]+) preparing: copies ([\d.]+) ms "
    r"[\d.]+ MB, textures ([\d.]+) ms [\d.]+ MB; ([\d.]+) on the\s+GPU; "
    r"([\d.]+) copying back [\d.]+ MB\), ([\d.]+) draws/frame, "
    r"dropped ([\d.]+) without a host pipeline \((\d+) being made\)"
    r"(?: and ([\d.]+) indirect)?"
)
FIELDS = ("fps", "frame_ms", "prepare_ms", "copies_ms", "texture_ms", "gpu_ms",
          "back_ms", "draws", "dropped", "making", "indirect_dropped")

def rows(text, min_draws=1000):
    # PowerShell can wrap native stderr records across physical lines.
    # Join continuations only until the next timestamped event.
    for line in re.split(r"(?=\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})", text):
        match = FRAME.search(" ".join(line.split()))
        if match:
            row = dict(zip(FIELDS, (float(v) if v is not None else float("nan") for v in match.groups())))
            if row["draws"] >= min_draws:
                yield row

def percentile(values, fraction):
    values = sorted(values)
    return values[max(0, math.ceil(fraction * len(values)) - 1)]

def summarize(data):
    result = {**{k: statistics.median([r[k] for r in data if math.isfinite(r[k])] or [float("nan")]) for k in FIELDS},
            "p95_interval_ms": percentile([r["frame_ms"] for r in data], .95),
            "intervals_over_100ms": sum(r["frame_ms"] > 100 for r in data)}
    valid_gpu = [r["gpu_ms"] for r in data if r["gpu_ms"] > 0]
    result["gpu_ms"] = statistics.median(valid_gpu) if valid_gpu else float("nan")
    result["gpu_timing_missing"] = len(data) - len(valid_gpu)
    return result

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("baseline", type=Path)
    p.add_argument("candidate", type=Path)
    p.add_argument("--min-draws", type=int, default=1000)
    p.add_argument("--skip", type=int, default=0, help="Skip this many qualifying intervals in each log")
    p.add_argument("--count", type=int, default=60, help="Maximum qualifying intervals to compare")
    p.add_argument("--last", action="store_true", help="Take the final count qualifying intervals, after skip")
    p.add_argument("--warm-only", action="store_true", help="Require zero pipelines being made in selected intervals")
    a = p.parse_args()
    if a.skip < 0 or a.count < 1 or a.min_draws < 0:
        p.error("invalid sample selection")
    def load(path):
        data = path.read_bytes()
        encoding = "utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
        eligible = [r for r in rows(data.decode(encoding, errors="replace"), a.min_draws) if not a.warm_only or r["making"] == 0][a.skip:]
        return eligible[-a.count:] if a.last else eligible[:a.count]
    base, cand = load(a.baseline), load(a.candidate)
    n = min(len(base), len(cand))
    if not n:
        p.error("No matched qualifying frame lines. Set VICTORY_LOG=info and capture stderr.")
    old, new = (summarize(base[-n:]), summarize(cand[-n:])) if a.last else (summarize(base[:n]), summarize(cand[:n]))
    print(f"Intervals compared per log: {n} (not individual presents)")
    print(f"{'Metric (median unless noted)':32} {'Baseline':>12} {'Candidate':>12}")
    for k in old:
        print(f"{k:32} {old[k]:12.2f} {new[k]:12.2f}")
    if old["gpu_timing_missing"] or new["gpu_timing_missing"]:
        print("Warning: zero GPU timings excluded; GPU comparison may be biased or unavailable.")
    print("Compare matching scenes, caches, hardware and draw counts; lower time with missing draws is not a valid win.")
    if n < 20:
        print("Warning: fewer than 20 qualifying intervals; noisy comparison.")
    if old['draws'] and abs(new['draws'] - old['draws']) / old['draws'] > .05:
        print("Warning: median draw counts differ by more than 5%; check scene and visual coverage.")
    if new['dropped'] > old['dropped']:
        print("Warning: median dropped draws increased; check rendering correctness.")
    if new['indirect_dropped'] > old['indirect_dropped']:
        print("Warning: median indirect drops increased; check rendering correctness.")

if __name__ == "__main__":
    main()
