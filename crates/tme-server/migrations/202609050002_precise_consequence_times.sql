-- Retire ambiguous whole-unit columns. Preserve historical timestamps by
-- converting their authored three-second units exactly once in this migration.
ALTER TABLE tme.player_kill_marks
    RENAME COLUMN assessed_logical_time TO assessed_logical_millis;
UPDATE tme.player_kill_marks
    SET assessed_logical_millis = assessed_logical_millis * 3000;
ALTER TABLE tme.pending_player_kill_consequences
    RENAME COLUMN assessed_logical_time TO assessed_logical_millis;
UPDATE tme.pending_player_kill_consequences
    SET assessed_logical_millis = assessed_logical_millis * 3000;
