#!/usr/bin/env python3
"""Keep runtime retirement checks strict when systemd skips unmet conditions."""

import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "ci_retirement", Path(__file__).resolve().parents[1] / "scripts/ci-retirement.py")
RETIREMENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RETIREMENT)


def stopped_service(**changes):
    values = {
        "LoadState": "loaded", "ActiveState": "inactive", "SubState": "dead",
        "MainPID": "0", "ControlPID": "0", "ExecMainStatus": "0", "Result": "success",
        "ExecMainStartTimestampMonotonic": "200", "ConditionResult": "yes",
        "ConditionTimestampMonotonic": "100",
    }
    return values | changes


def show_result(values, returncode=0):
    return subprocess.CompletedProcess(
        ["systemctl", "show"], returncode,
        "".join(f"{key}={value}\n" for key, value in values.items()), "")


class RuntimeRetirementTests(unittest.TestCase):
    def check_start(self, after, returncode=0, before=None):
        before = before if before is not None else stopped_service()
        # Read both snapshots through the same fail-closed parser as the actual
        # acceptance flow; a missing unit or failed status read is not a refusal.
        with patch.object(RETIREMENT, "command", side_effect=[show_result(before), show_result(after)]):
            initial = RETIREMENT.service(RETIREMENT.RUNTIME_UNIT)
            final = RETIREMENT.service(RETIREMENT.RUNTIME_UNIT)
        RETIREMENT.runtime_restart_refused(initial, final, returncode)

    def test_fresh_condition_skip_accepts_success_without_runtime_execution(self):
        for last_start in ("200", "0"):
            with self.subTest(last_start=last_start):
                self.check_start(stopped_service(ConditionResult="no", ConditionTimestampMonotonic="300",
                                                ExecMainStartTimestampMonotonic=last_start))

    def test_explicit_start_failure_before_exec_remains_accepted(self):
        self.check_start(stopped_service(ActiveState="failed", SubState="failed", Result="exit-code"), returncode=1)

    def test_success_requires_fresh_failed_condition_evidence(self):
        cases = (
            stopped_service(),
            stopped_service(ConditionTimestampMonotonic="300"),
            stopped_service(ConditionResult="no"),
            stopped_service(ConditionResult="no", ConditionTimestampMonotonic="0"),
            stopped_service(ConditionResult="no", ConditionTimestampMonotonic="99"),
        )
        for after in cases:
            with self.subTest(after=after), self.assertRaises(RETIREMENT.Failure):
                self.check_start(after)

    def test_zero_pids_do_not_hide_a_runtime_that_executed_and_exited(self):
        for returncode in (0, 1):
            with self.subTest(returncode=returncode), self.assertRaises(RETIREMENT.Failure):
                self.check_start(stopped_service(ConditionResult="no", ConditionTimestampMonotonic="400",
                                                ExecMainStartTimestampMonotonic="300"), returncode)

    def test_live_process_or_start_transition_is_never_a_refusal(self):
        changes = (
            {"MainPID": "123"}, {"ControlPID": "123"},
            {"ActiveState": "active", "SubState": "running"},
            {"ActiveState": "activating", "SubState": "auto-restart"},
            {"ActiveState": "deactivating", "SubState": "stop-sigterm"},
            {"SubState": "start-pre"},
        )
        for state in changes:
            for returncode in (0, 1):
                with self.subTest(state=state, returncode=returncode), self.assertRaises(RETIREMENT.Failure):
                    self.check_start(stopped_service(ConditionResult="no", ConditionTimestampMonotonic="300", **state),
                                     returncode)

    def test_missing_or_unloadable_unit_is_not_successful_retirement(self):
        for load_state in ("not-found", "error", "masked", ""):
            for returncode in (0, 1):
                with self.subTest(load_state=load_state, returncode=returncode), self.assertRaises(RETIREMENT.Failure):
                    self.check_start(stopped_service(LoadState=load_state, ConditionResult="no",
                                                    ConditionTimestampMonotonic="300"), returncode)

    def test_interrupted_systemctl_is_not_a_confirmed_start_refusal(self):
        with self.assertRaises(RETIREMENT.Failure):
            self.check_start(stopped_service(ConditionResult="no", ConditionTimestampMonotonic="300"), returncode=-15)

    def test_status_reads_must_succeed_and_be_complete_unambiguous_numbers(self):
        valid = show_result(stopped_service())
        outputs = [
            show_result(stopped_service(), returncode=1),
            subprocess.CompletedProcess([], 0, "", ""),
            subprocess.CompletedProcess([], 0, valid.stdout + "MainPID=0\n", ""),
            subprocess.CompletedProcess([], 0, valid.stdout.replace("ControlPID=0\n", ""), ""),
            subprocess.CompletedProcess([], 0, "x" * 4097, ""),
        ]
        for key in ("MainPID", "ControlPID", "ExecMainStartTimestampMonotonic", "ConditionTimestampMonotonic"):
            for bad_number in ("", "-1", "unknown"):
                outputs.append(show_result(stopped_service(**{key: bad_number})))
        outputs.append(show_result(stopped_service(ConditionResult="unknown")))
        for result in outputs:
            with self.subTest(stdout=result.stdout), patch.object(RETIREMENT, "command", return_value=result), \
                    self.assertRaises(RETIREMENT.Failure):
                RETIREMENT.service(RETIREMENT.RUNTIME_UNIT)


if __name__ == "__main__":
    unittest.main()
