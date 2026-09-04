class_name WorldViewSeam
extends Control

## The one seam between the world shell and whatever draws the world.
##
## The shell owns intent, authority reconciliation, and every HUD surface. A
## view owns exactly two things the shell cannot do for itself: turning an
## authoritative frame into something a player can see, and turning a pointer
## position into one of [WorldTargets]' semantic targets. Everything a view is
## asked for below is expressed in world squares and frame rows — never in
## meshes, cameras, or pixels — so a future renderer lands behind this contract
## without the shell learning it exists.
##
## [GridWorldView] is the current implementation. It draws the frame as a square
## lattice of flat colours and resolves pointers against the exact rectangles it
## drew — real targeting, no art. A pixel-native renderer substitutes for it
## behind this contract without the shell learning that it happened, which is
## what the contract is for.
##
## Contract for an implementation:
## [br]- [method present_frame] is the only way authority enters a view.
## [br]- Targets a view emits or returns must come from [WorldTargets], so the
##   director and the tray see one identity space no matter what drew it.
## [br]- A view never sends a command and never mutates authoritative state; it
##   emits pointer facts and the shell decides what they mean.
## [br]- Pointer signals carry a position in [method pointer_surface] local
##   space, which the shell converts when it needs a global one.

## A press began on a target. `display_position` is in [method pointer_surface]
## local space.
signal semantic_primary_pressed(target: Dictionary, display_position: Vector2)

## The press that began with [signal semantic_primary_pressed] ended.
signal semantic_primary_released(target: Dictionary, display_position: Vector2)

## The pointer moved, with the target currently under it (empty for none).
signal semantic_pointer_moved(target: Dictionary, display_position: Vector2)

## A secondary press landed on a target — the context route.
signal semantic_secondary_pressed(target: Dictionary, display_position: Vector2)


## Installs one authoritative frame. An empty frame means "no authority": a view
## must present that as absence, never as stale world.
func present_frame(_frame: Dictionary, _frame_generation: int) -> void:
	pass


## Installs the action interval the view is drawing inside of, as [ActionCooldown] accounts for
## it. It carries no authority a view may act on: readiness, logical time, and
## the wait are the frame's, already installed through [method present_frame].
## What this adds is only how far into the current action interval presentation has got, so
## a view can spread a step across it instead of snapping. A view that does not
## animate ignores it entirely.
func present_cooldown(_state: Dictionary) -> void:
	pass


## Drops all presented state and returns to the no-authority presentation.
func clear() -> void:
	pass


## The square the frame is observed from.
func observation_center() -> Vector2i:
	return Vector2i.ZERO


## The semantic target at a square, or an empty dictionary when the square is
## not part of the current frame.
func semantic_target_for_coordinate(_coordinate: Vector2i) -> Dictionary:
	return {}


## The semantic target under a pointer position, or an empty dictionary. A view
## with no drawn geometry has no honest answer here and returns empty; a view
## that drew the frame answers from the geometry it drew, never from the nearest
## anchor to the pointer.
func semantic_target_for_display_position(_display_position: Vector2) -> Dictionary:
	return {}


## The control whose local space pointer positions are expressed in.
func pointer_surface() -> Control:
	return self


## Shows a movement path the player has drafted but the server has not answered.
func show_pending(_start: Vector2i, _path: Array[String]) -> void:
	pass


## Shows an authoritative path preview result.
func show_preview(_preview: Dictionary) -> void:
	pass


## Drops any pending or previewed movement presentation.
func clear_interaction() -> void:
	pass


## Marks the reach grid as transiently active for the duration of a press.
func set_reach_grid_transient_active(_active: bool) -> void:
	pass


## Cycles the player's reach-grid preference.
func toggle_grid() -> void:
	pass


## The reach grid's current control state, as a lower-snake token.
func grid_control_state() -> String:
	return "unavailable"


## Presents one feedback cue in the world and returns what was presented, so the
## shell can record the fact without knowing how it was drawn.
func present_feedback(_kind: String) -> Dictionary:
	return {}
