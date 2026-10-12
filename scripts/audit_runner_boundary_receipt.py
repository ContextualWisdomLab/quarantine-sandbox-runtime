"""Offline consistency audit; retained observations never authorize CI leasing."""

import argparse
import hashlib
import json
import os
import stat

SUMMARY_LIMIT = 262144
SERIAL_LIMIT = 1048576
GUEST_KEYS = {
    "transaction",
    "hostname",
    "pristine_paths",
    "host_gateway",
    "host_lan",
    "external",
}


def _read_bounded(path, limit):
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("regular_file_required")
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise ValueError("input_too_large")
    return raw


def _unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate_key")
        result[key] = value
    return result


def _reject_constant(_value):
    raise ValueError("nonfinite_number")


def _json(raw):
    value = json.loads(
        raw.decode("utf-8"),
        object_pairs_hook=_unique_pairs,
        parse_constant=_reject_constant,
    )
    pending = [(value, 0)]
    while pending:
        item, depth = pending.pop()
        if depth > 16:
            raise ValueError("json_too_deep")
        if isinstance(item, dict):
            pending.extend(
                (child, depth + 1) for pair in item.items() for child in pair
            )
        elif isinstance(item, list):
            pending.extend((child, depth + 1) for child in item)
        elif isinstance(item, str):
            item.encode("utf-8", errors="strict")
        elif isinstance(item, float):
            raise TypeError("integer_receipt_required")
    return value


def audit_receipt(summary, control_serial, denied_serial, expected_summary_sha256):
    """Return a limited observation or a fixed, non-authoritative rejection."""
    try:
        return _audit_receipt(
            summary, control_serial, denied_serial, expected_summary_sha256
        )
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        AttributeError,
        RecursionError,
        OverflowError,
    ) as error:
        raise ValueError("receipt_rejected") from error


def _audit_receipt(summary, control_serial, denied_serial, expected_summary_sha256):
    """Join a caller-pinned summary to serial observations, not operator trust."""
    raw = _read_bounded(summary, SUMMARY_LIMIT)
    if hashlib.sha256(raw).hexdigest() != expected_summary_sha256:
        raise ValueError("summary_digest_mismatch")
    data = _json(raw)
    if data["passed_limited_vm_boundary"] is not True or any(
        data[key] is not False
        for key in ("whole_cidr_proof", "full_clean_job", "registered_ci_accepted")
    ):
        raise ValueError("limited_scope_required")
    settlement = data["settlement"]
    if (
        any(
            settlement[key] is not True
            for key in ("passed", "existing_pids_preserved", "receipt_fsynced")
        )
        or settlement["owned_pids"] != []
        or settlement["errors"] != []
    ):
        raise ValueError("cleanup_not_verified")
    rows = data["results"]
    transaction = rows["control"]["transaction"]
    if (
        not isinstance(transaction, str)
        or len(transaction) != 32
        or any(character not in "0123456789abcdef" for character in transaction)
        or rows["denied"]["transaction"] != transaction
    ):
        raise ValueError("transaction_mismatch")
    identities = []
    for role, path in (("control", control_serial), ("denied", denied_serial)):
        row = rows[role]
        identity = row["qemu_identity"]
        process = identity["process"]
        unit = "sdp-vm-boundary-" + role + "-20261003.service"
        cgroup = "/system.slice/" + unit
        invocation = identity["invocation"]
        if (
            type(identity["pid"]) is not int
            or identity["pid"] <= 0
            or (type(process["uid"]) is not int or process["uid"] <= 0)
            or identity["cgroup"] != cgroup
            or process["cgroup"] != "0::" + cgroup
            or (process["exe"] != "/usr/bin/qemu-system-x86_64")
            or not isinstance(process["start"], str)
            or not process["start"].isascii()
            or (not process["start"].isdigit() or int(process["start"]) <= 0)
            or not isinstance(invocation, str)
            or len(invocation) != 32
            or any(character not in "0123456789abcdef" for character in invocation)
        ):
            raise ValueError("process_identity_invalid")
        identities.append(identity)
        if type(row["wrapper_exit"]) is not int or row["wrapper_exit"] != 0:
            raise ValueError("wrapper_failed")
        after = row["unit_after_exit"]
        expected_after = {
            "MainPID": "0",
            "ControlGroup": "",
            "IPAddressDeny": "",
            "User": "",
            "Description": unit,
            "LoadState": "not-found",
            "ActiveState": "inactive",
            "InvocationID": "",
        }
        key = "False" if role == "control" else "True"
        if (
            after != expected_after
            or settlement["units"][key] != after
            or (
                type(settlement["wrappers"][key]) is not int
                or settlement["wrappers"][key] != 0
            )
        ):
            raise ValueError("unit_cleanup_invalid")
        expected_hits = ["host_gateway", "host_lan"] if role == "control" else []
        if row["fixture_hits"] != expected_hits:
            raise ValueError("fixture_hits_invalid")
        attachments = row["attachments"]
        if role == "control":
            if attachments != {}:
                raise ValueError("control_attachments_invalid")
        elif (
            not isinstance(attachments, dict)
            or set(attachments) != {"0", "1"}
            or any(
                not isinstance(values, list)
                or not values
                or len(values) > 64
                or any(type(value) is not int or value <= 0 for value in values)
                or len(set(values)) != len(values)
                for values in attachments.values()
            )
        ):
            raise ValueError("attachments_invalid")
        expected_result = "connected" if role == "control" else "TimeoutError"
        if (
            row["hostname"] != "sdp-vm-" + role
            or row["pristine_paths"] is not True
            or (
                row["external"] != "connected"
                or row["host_gateway"] != expected_result
                or row["host_lan"] != expected_result
            )
        ):
            raise ValueError("point_observation_invalid")
        serial = _read_bounded(path, SERIAL_LIMIT)
        row = data["results"][role]
        if hashlib.sha256(serial).hexdigest() != row["serial_sha256"]:
            raise ValueError("serial_digest_mismatch")
        lines = [
            line[len(b"SDP_VM_RESULT=") :]
            for line in serial.splitlines()
            if line.startswith(b"SDP_VM_RESULT=")
        ]
        if len(lines) != 1:
            raise ValueError("guest_result_count")
        guest = _json(lines[0])
        if (
            not isinstance(guest, dict)
            or set(guest) != GUEST_KEYS
            or any(
                type(row[key]) is not type(value) or row[key] != value
                for key, value in guest.items()
            )
        ):
            raise ValueError("guest_summary_mismatch")
    if any(identities[0][key] == identities[1][key] for key in ("pid", "invocation")):
        raise ValueError("process_identity_reused")
    return {
        "observation_consistent": True,
        "lease_allowed": False,
        "operator_authenticated": False,
        "scope": "limited_vm_point_observation",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", required=True)
    parser.add_argument("--control-serial", required=True)
    parser.add_argument("--denied-serial", required=True)
    parser.add_argument("--expected-summary-sha256", required=True)
    args = parser.parse_args()
    try:
        result = audit_receipt(
            args.summary,
            args.control_serial,
            args.denied_serial,
            args.expected_summary_sha256,
        )
    except ValueError:
        print(
            json.dumps(
                {
                    "observation_consistent": False,
                    "lease_allowed": False,
                    "operator_authenticated": False,
                    "scope": "limited_vm_point_observation",
                    "error": "receipt_rejected",
                },
                sort_keys=True,
            )
        )
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
