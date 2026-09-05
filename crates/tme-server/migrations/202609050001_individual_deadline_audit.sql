-- Applied migrations and historical audit events remain immutable. The old
-- facet_tick label is retained only for existing history; runtime writes use
-- facet_deadlines for independently due work.
ALTER TABLE tme.audit_events DROP CONSTRAINT audit_events_action_check;
ALTER TABLE tme.audit_events ADD CONSTRAINT audit_events_action_check CHECK (
    action IN (
        'account_create', 'account_set_password', 'login', 'logout',
        'admit', 'command', 'restore_fence', 'facet_tick', 'facet_deadlines',
        'facet_presence', 'mark_assess', 'mark_expire', 'mark_forgive',
        'session_expire'
    )
);
