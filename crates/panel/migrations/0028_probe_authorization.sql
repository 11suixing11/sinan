-- Existing targets have no recorded authorization. Keep their IDs and all history,
-- but require an explicit confirmation before any new periodic measurements.
UPDATE network_probes SET spec = jsonb_set(spec, '{enabled}', 'false'::jsonb)
    WHERE NOT (spec ? 'monitoring');
UPDATE latency_tasks SET spec = jsonb_set(spec, '{enabled}', 'false'::jsonb),
    default_enabled = FALSE, revision = revision + 1
    WHERE NOT (spec ? 'monitoring');
