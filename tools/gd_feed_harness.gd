extends RefCounted
## Test harness only. FIXLOG 2026-11-19: wait for completed ticks, not a poll count.
## Injected monotonic clock/yield let toy workers exercise timeout without sleeping.

const TARGET_TICKS := 6000
const BATCH_TICKS := 20
const TIMEOUT_MS := 30000
const STALL_MS := 5000

static func collect(sim, threaded: bool, now: Callable, pause: Callable,
		timeout_ms: int = TIMEOUT_MS, stall_ms: int = STALL_MS) -> Dictionary:
	var start: int = sim.tick()
	var began: int = now.call()
	var last_progress := began
	var previous := start
	var result := {"start": start, "ticks": start, "asked": 0, "rows": 0,
		"while_running": 0, "error": ""}
	while true:
		var tick: int = sim.tick()
		var elapsed: int = now.call()
		result.ticks = tick
		if tick < previous or tick > start + TARGET_TICKS:
			result.error = "unexpected completed tick"
			return result
		if tick > previous:
			previous = tick
			last_progress = elapsed
		if elapsed - began >= timeout_ms:
			result.error = "bounded completion timeout"
			return result
		if tick == start + TARGET_TICKS:
			break
		if elapsed - last_progress >= stall_ms:
			result.error = "worker stalled without completed-tick progress"
			return result
		# One outstanding batch, as in the original guard; drain on every live poll.
		if result.asked < TARGET_TICKS and tick >= start + result.asked:
			sim.step(BATCH_TICKS)
			result.asked += BATCH_TICKS
		if threaded:
			pause.call()
		result.rows += sim.drain_feed().size()
	result.while_running = result.rows
	# No blocking sync on timeout/stall. Only settle after all requested ticks completed.
	sim.sync()
	result.rows += sim.drain_feed().size()
	result.ticks = sim.tick()
	return result

static func parity_errors(a: Dictionary, b: Dictionary) -> Array[String]:
	var errors: Array[String] = []
	for run in [a, b]:
		if run.error != "" or run.asked != TARGET_TICKS or run.ticks != run.start + TARGET_TICKS:
			errors.append("run did not complete exactly 6000 ticks: %s; asked %d, completed %d" % [run.error, run.asked, run.ticks - run.start])
	if a.ticks != b.ticks:
		errors.append("both runs must reach the same absolute tick")
	# Unequal durations are a completion failure, never a misleading feed comparison.
	if not errors.is_empty():
		return errors
	if b.rows <= 0:
		errors.append("the run must make feed lines")
	if a.rows != b.rows:
		errors.append("feed row parity mismatch: %d vs %d" % [a.rows, b.rows])
	if a.while_running * 2 < a.rows:
		errors.append("feed did not arrive while the worker was live")
	return errors
