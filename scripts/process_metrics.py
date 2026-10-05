"""CPU, resident/private memory, threads and I/O counters without third-party modules.

Windows: native process APIs. Linux: /proc. CPU 100% means one logical core.
"""
import ctypes
from ctypes import wintypes
import os
from pathlib import Path
import time


class ProcessMetrics:
    def __init__(self, pid):
        self.pid = pid
        self.previous = None
        self.handle = None
        if os.name == "nt":
            self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            self.psapi = ctypes.WinDLL("psapi", use_last_error=True)
            self.kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
            self.kernel.OpenProcess.restype = wintypes.HANDLE
            self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
            self.kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
            self.kernel.GetProcessIoCounters.argtypes = [wintypes.HANDLE, ctypes.c_void_p]
            self.psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD]
            self.handle = self.kernel.OpenProcess(0x1000 | 0x0010, False, pid)
            if not self.handle:
                raise ctypes.WinError(ctypes.get_last_error())
        elif not Path("/proc").exists():
            raise RuntimeError("Process measurements support Windows and Linux")

    def sample(self):
        if os.name == "nt":
            class Memory(ctypes.Structure):
                _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD)] + [(name, ctypes.c_size_t) for name in
                    ("peak_rss", "rss", "peak_paged", "paged", "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile", "private")]

            class IO(ctypes.Structure):
                _fields_ = [(name, ctypes.c_ulonglong) for name in ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]

            times = [wintypes.FILETIME() for _ in range(4)]
            if not self.kernel.GetProcessTimes(self.handle, *[ctypes.byref(t) for t in times]):
                raise ctypes.WinError(ctypes.get_last_error())
            cpu = sum((t.dwHighDateTime << 32) | t.dwLowDateTime for t in times[2:]) / 10_000_000
            memory = Memory()
            memory.cb = ctypes.sizeof(memory)
            if not self.psapi.GetProcessMemoryInfo(self.handle, ctypes.byref(memory), memory.cb):
                raise ctypes.WinError(ctypes.get_last_error())
            io = IO()
            if not self.kernel.GetProcessIoCounters(self.handle, ctypes.byref(io)):
                raise ctypes.WinError(ctypes.get_last_error())
            result = {"cpu_seconds": cpu, "rss_bytes": memory.rss, "private_bytes": memory.private,
                      "io_read_bytes": io.read_bytes, "io_write_bytes": io.write_bytes}
        else:
            proc = Path("/proc") / str(self.pid)
            # comm may include spaces or parentheses; fields after its final ')' start at state.
            fields = (proc / "stat").read_text().rsplit(")", 1)[1].split()
            cpu = (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")
            io = dict(line.split(":", 1) for line in (proc / "io").read_text().splitlines())
            result = {"cpu_seconds": cpu, "rss_bytes": int(fields[21]) * os.sysconf("SC_PAGE_SIZE"),
                      "private_bytes": None, "threads": int(fields[17]),
                      "io_read_bytes": int(io["read_bytes"]), "io_write_bytes": int(io["write_bytes"])}
        now = time.monotonic()
        result["cpu_percent_one_core"] = None
        if self.previous and now > self.previous[0]:
            then, before = self.previous
            result["cpu_percent_one_core"] = max(0, (cpu - before) / (now - then) * 100)
        self.previous = now, cpu
        result["pid"] = self.pid
        return result

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


def distribution(values):
    if not values:
        return {"count": 0, "mean": None, "p50": None, "p95": None, "p99": None, "max": None}
    values = sorted(values)
    def percentile(fraction):
        position = (len(values) - 1) * fraction
        lower = int(position)
        upper = min(lower + 1, len(values) - 1)
        return values[lower] + (values[upper] - values[lower]) * (position - lower)
    return {"count": len(values), "mean": sum(values) / len(values), "p50": percentile(.5),
            "p95": percentile(.95), "p99": percentile(.99), "max": values[-1]}


def summarize_ticks(path):
    records = []
    if path.exists():
        import json
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue  # a forced shutdown may leave a partial final record
    fields = ("tick_interval_ms", "processing_ms", "network_receive_ms", "commands_ms", "simulation_ms",
              "events_ms", "snapshots_ms", "network_send_ms", "cleanup_ms", "observer_ms_previous_tick")
    allocation_records = [r["allocations"] for r in records if r.get("allocations", {}).get("enabled")]
    allocations = None
    if allocation_records:
        phases = ("network_receive", "commands", "simulation", "events", "snapshots", "network_send", "cleanup")
        allocations = {
            "samples": len(allocation_records),
            "first_totals": allocation_records[0]["totals"],
            "last_totals": allocation_records[-1]["totals"],
            "last_system_totals": allocation_records[-1].get("systems", []),
            "phases": {phase: {field: sum(r[phase][field] for r in allocation_records)
                       for field in ("allocated_bytes", "freed_bytes", "allocation_calls", "net_bytes")} for phase in phases},
        }
    return {"samples": len(records), "allocations": allocations,
            "last_buffer_stats": records[-1].get("buffers") if records else None,
            "max_flow_fields": max((r.get("flow_fields", 0) for r in records), default=None),
            "timings_ms": {field: distribution([r[field] for r in records]) for field in fields},
            "over_budget_ticks": sum(r["over_budget"] for r in records),
            "max_active_entities": max((r["active_entities"] for r in records), default=None),
            "max_allocated_entities": max((r["allocated_entities"] for r in records), default=None),
            "network_totals": {key: sum(r["network"][key] for r in records) for key in
                ("received_messages", "received_payload_bytes", "sent_messages", "sent_payload_bytes", "decode_errors", "inputs", "shop_actions", "lobby_messages")}}
