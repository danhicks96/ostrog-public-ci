extends SceneTree
## Pure harness tests: no native extension, game assets, threads or real sleeps.
const Harness := preload("gd_feed_harness.gd")
var failures := 0

class Clock:
	extends RefCounted
	var time := 0
	var quantum := 1
	func now() -> int:
		return time
	func pause() -> void:
		time += quantum

class Worker:
	extends RefCounted
	var current := 100
	var pending := 0
	var polls := 0
	var polls_per_batch := 15
	var threaded := true
	var stalled := false
	var drop_one := false
	var dropped := false
	var queued_rows := 0
	var sync_calls := 0
	func tick() -> int:
		return current
	func step(n: int) -> void:
		assert(pending == 0)
		pending = n
		if not threaded:
			complete()
	func complete() -> void:
		current += pending
		pending = 0
		if (current - 100) % 100 == 0:
			if drop_one and not dropped:
				dropped = true
			else:
				queued_rows += 1
	func drain_feed() -> Array:
		if threaded and pending > 0 and not stalled:
			polls += 1
			if polls % polls_per_batch == 0:
				complete()
		var rows := []
		rows.resize(queued_rows)
		queued_rows = 0
		return rows
	func sync() -> void:
		sync_calls += 1
		if pending > 0:
			complete()

func expect(ok: bool, label: String) -> void:
	if not ok:
		failures += 1
		push_error(label)

func run_worker(worker: Worker, clock: Clock) -> Dictionary:
	return Harness.collect(worker, worker.threaded, clock.now, clock.pause)

# Reproduce the old guard's fixed poll cap with the same toy worker and final settle.
func legacy_collect(worker: Worker, clock: Clock) -> int:
	var start := worker.tick()
	var asked := 0
	var polls := 0
	while worker.tick() < start + 6000 and polls < 4000:
		polls += 1
		if asked < 6000 and worker.tick() >= start + asked:
			worker.step(20)
			asked += 20
		clock.pause()
		worker.drain_feed()
	worker.sync()
	return worker.tick() - start

func _initialize() -> void:
	var old_completed := legacy_collect(Worker.new(), Clock.new())
	expect(old_completed < 6000, "old poll cap demonstrably compares an incomplete run")
	print("FEED_HARNESS_INFO legacy poll cap completed %d of 6000 toy ticks" % old_completed)
	var live := Worker.new()
	var clock := Clock.new()
	var a := run_worker(live, clock)
	var inline_worker := Worker.new()
	inline_worker.threaded = false
	var b := run_worker(inline_worker, Clock.new())
	expect(live.polls > 4000 and a.ticks == 6100 and a.asked == 6000, "slow progressing worker must complete beyond old poll cap")
	expect(Harness.parity_errors(a, b).is_empty(), "completed equal-duration feed must pass")
	var frozen := Worker.new()
	frozen.stalled = true
	var stalled := run_worker(frozen, Clock.new())
	expect(stalled.error.contains("stalled") and frozen.sync_calls == 0, "stalled worker fails without blocking sync")
	expect(not Harness.parity_errors(stalled, b).is_empty(), "stalled worker cannot pass parity")
	var slow := Worker.new()
	slow.polls_per_batch = 2
	var slow_clock := Clock.new()
	slow_clock.quantum = 100
	var timed := run_worker(slow, slow_clock)
	expect(timed.error.contains("timeout") and slow.sync_calls == 0, "continuing progress still has total deadline")
	var missing := Worker.new()
	missing.drop_one = true
	var mismatch := run_worker(missing, Clock.new())
	expect(mismatch.ticks == b.ticks and Harness.parity_errors(mismatch, b).any(func(s): return s.contains("parity mismatch")), "equal duration with missing feed must fail")
	var late := a.duplicate()
	late.while_running = 0
	expect(Harness.parity_errors(late, b).any(func(s): return s.contains("live")), "feed only after finishing must fail")
	var incomplete := a.duplicate()
	incomplete.ticks -= 20
	incomplete.rows -= 1
	var errors := Harness.parity_errors(incomplete, b)
	expect(not errors.is_empty() and not errors.any(func(s): return s.contains("parity mismatch")), "incomplete duration fails before feed comparison")
	print("FEED_HARNESS_TEST 9 checks, %d failed" % failures)
	quit(1 if failures else 0)
