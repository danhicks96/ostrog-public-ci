## One place that knows which panels exist, which are open, and which is on top.
##
## Before this, each panel was shown or hidden by a bare `.visible = ...` written at
## whichever site happened to need it - thirteen of them in main.gd alone - with nothing
## tracking z-order. Two panels open at once therefore stacked in the order they were
## built rather than the order they were opened, and there was no single place to ask
## "is anything open?" (#351).
##
## This is deliberately thin: it does not own layout (rules.gd still does, and #221/#246
## depend on those numbers) and it does not decide what a panel contains. It owns
## three things only: which panels exist, which are open, and which is topmost.
class_name PanelManager
extends RefCounted

## A panel was opened.
signal opened(id: StringName)
## A panel was closed.
signal closed(id: StringName)

var _panels: Dictionary = {}
## A panel's group, if it has one: opening a panel closes the other open panels of its group.
## main.gd puts the centred reading panels in one group, so one is read at a time and none shows
## through another's backing (D92, FIXLOG 2026-11-14).
var _group: Dictionary = {}
## Open panels in the order they were opened; the last is the one on top.
var _open: Array[StringName] = []


## Add a panel under `id`, in `group` if one is given. If it is already visible it is treated
## as open, so registering does not change what is on screen.
func register(id: StringName, node: Control, group: StringName = &"") -> void:
	if node == null:
		push_warning("PanelManager: refusing to register a null panel '%s'" % id)
		return
	_panels[id] = node
	_group[id] = group
	if node.visible and not _open.has(id):
		_open.append(id)


func has(id: StringName) -> bool:
	return _panels.has(id)


func is_open(id: StringName) -> bool:
	return _open.has(id)


## Show a panel, remember it, and raise it above every other panel on its layer.
func open(id: StringName) -> void:
	if not _panels.has(id):
		push_warning("PanelManager: no panel registered as '%s'" % id)
		return
	var g: StringName = _group.get(id, &"")
	if g != &"":
		for other in _open.duplicate():
			if other != id and _group.get(other, &"") == g:
				close(other)
	var n: Control = _panels[id]
	if not n.visible:
		n.visible = true
	_open.erase(id)
	_open.append(id)
	_raise(n)
	opened.emit(id)


## Hide a panel and forget it.
func close(id: StringName) -> void:
	if not _panels.has(id):
		return
	var n: Control = _panels[id]
	n.visible = false
	_open.erase(id)
	closed.emit(id)


## Open if closed, close if open. Returns the new state.
func toggle(id: StringName) -> bool:
	if is_open(id):
		close(id)
		return false
	open(id)
	return true


func close_all() -> void:
	for id in _open.duplicate():
		close(id)


## The id of the panel on top, or &"" when nothing is open.
func top() -> StringName:
	return _open[-1] if not _open.is_empty() else &""


func open_ids() -> Array[StringName]:
	return _open.duplicate()


## Whether anything is open at all: one answer for "should Esc clear the screen?".
func any_open() -> bool:
	return not _open.is_empty()


func _raise(n: Control) -> void:
	var p := n.get_parent()
	if p != null and n.get_index() != p.get_child_count() - 1:
		p.move_child(n, -1)
