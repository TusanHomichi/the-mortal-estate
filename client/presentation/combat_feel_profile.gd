class_name CombatFeelProfile
extends Resource

@export_range(1, 2000, 1) var double_activation_window_msec: int = 420
@export_range(0, 2000, 1) var melee_wind_up_msec: int = 180
@export_range(0, 2000, 1) var melee_minimum_payoff_msec: int = 280
@export_range(0, 2000, 1) var ranged_release_msec: int = 160
@export_range(0, 2000, 1) var ranged_minimum_payoff_msec: int = 260
@export_range(0, 3000, 1) var spell_chant_msec: int = 600
@export_range(0, 2000, 1) var spell_release_msec: int = 180
@export_range(0, 2000, 1) var spell_minimum_payoff_msec: int = 300
@export_range(0, 3000, 1) var visual_tail_cap_msec: int = 700
