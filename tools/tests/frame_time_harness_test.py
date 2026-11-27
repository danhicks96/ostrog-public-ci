#!/usr/bin/env python3
"""Guard the real-client frame-time harness required by WP-06 slice 6.1."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "tools" / "perf" / "frame_time.gd"
PLAN = ROOT / "docs" / "plan" / "WP-06-renderer-art.md"


def main() -> None:
    source = HARNESS.read_text(encoding="utf-8")
    required = (
        "DisplayServer.window_set_size(Vector2i(1920, 1080))",
        "DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)",
        "RenderingServer.frame_post_draw",
        "Performance.TIME_PROCESS",
        'ProjectSettings.get_setting("rendering/renderer/rendering_method", "unknown")',
        "sim.last_step_ms()",
        "refresh_accum > 0.05",
        "rendering forever without ever entering _measure_one_frame",
        "worker_busy_ms",
        '"PASS" if pass_gate',
        "output.store_line(\"frame,wall_ms,process_ms,render_wait_ms,sim_thread_ms,worker_busy_ms",
    )
    for needle in required:
        assert needle in source, f"frame-time harness lost requirement: {needle}"
    if PLAN.is_file():
        plan = PLAN.read_text(encoding="utf-8")
        assert (
            "Status: open (6.1 complete" in plan
            or "Status: open (6.2 in progress" in plan
            or "Status: open (6.2 complete" in plan
            or "Status: open (6.3 complete" in plan
        )
        assert "Depends on: no dependency for 6.1-6.4" in plan
    elif (ROOT / "tools" / "publish").is_dir():
        raise FileNotFoundError(f"private renderer plan missing: {PLAN}")
    else:
        print("private renderer plan absent from public snapshot; harness checked")
    print("frame-time harness guard OK")


if __name__ == "__main__":
    main()
