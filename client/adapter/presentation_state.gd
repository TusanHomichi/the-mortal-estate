class_name PresentationState
extends RefCounted

const MAX_TRANSIENT_LOG_ENTRIES: int = 200

var focus_origin: String = ""
var modal_origin: String = ""
var hover_identity: String = ""
var movement_batch: Array[String] = []
var movement_batch_started_msec: int = 0
var local_selection: String = ""
var text_scale_percent: int = 100
var binding_preferences: Dictionary = {}
var animation_handles: Dictionary = {}
var transient_log: Array[String] = []
var feedback_presenter: FeedbackPresenter = FeedbackPresenter.new()


static func actor_is_present(actor: Dictionary) -> bool:
	var life_state: Variant = actor.get("life_state")
	return life_state is String and life_state != "dead"


func append_transient(message: String) -> void:
	transient_log.append(message)
	while transient_log.size() > MAX_TRANSIENT_LOG_ENTRIES: transient_log.pop_front()


func discard() -> void:
	focus_origin = ""
	modal_origin = ""
	hover_identity = ""
	movement_batch.clear()
	movement_batch_started_msec = 0
	local_selection = ""
	animation_handles.clear()
	transient_log.clear()
	feedback_presenter.discard()


func debug_summary() -> Dictionary:
	return {"focus_origin": focus_origin, "modal_origin": modal_origin, "hover_identity": hover_identity, "movement_batch_size": movement_batch.size(), "local_selection": local_selection, "text_scale_percent": text_scale_percent, "transient_log_size": transient_log.size(), "feedback_entries": feedback_presenter.feedback_entries.size(), "chat_entries": feedback_presenter.chat_entries.size()}
