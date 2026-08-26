use super::*;

impl Engine {
    pub(super) fn physical_attack_error_reason(error: &StepError) -> ActionBlockedReasonV1 {
        let message = error.message();
        if message.contains("not visible") {
            ActionBlockedReasonV1::BlockedBySight
        } else if message == "protected_target_requires_confirmation" {
            ActionBlockedReasonV1::ProtectedTargetRequiresConfirmation
        } else if message == "invalid_hostile_target" {
            ActionBlockedReasonV1::InvalidHostileTarget
        } else if message.contains("not enough stamina") {
            ActionBlockedReasonV1::InsufficientStamina
        } else if message.contains("bow is not nocked") {
            ActionBlockedReasonV1::BowNotNocked
        } else if message.starts_with("fight target is out of range")
            || message.starts_with("kick target is out of range")
        {
            ActionBlockedReasonV1::NotEngaged
        } else if message.contains("out of range") {
            ActionBlockedReasonV1::OutOfRange
        } else if message.contains("is not a weapon") {
            ActionBlockedReasonV1::RightHandNotWeapon
        } else if message.contains("does not support") {
            ActionBlockedReasonV1::PhysicalModeNotSupported
        } else if message.contains("not alive") {
            ActionBlockedReasonV1::ActorNotLiving
        } else {
            ActionBlockedReasonV1::InvalidTarget
        }
    }
}
