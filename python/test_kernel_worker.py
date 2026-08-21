import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


def _c_level_writer_available():
    for name in ("numpy", "matplotlib"):
        try:
            __import__(name)
            return True
        except Exception:
            continue
    return False


class KernelWorkerTests(unittest.TestCase):
    def _spawn(self, cwd=None):
        worker = subprocess.Popen(
            [sys.executable, str(Path(__file__).with_name("kernel_worker.py"))],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            cwd=cwd,
        )
        ready = json.loads(worker.stdout.readline())
        self.assertEqual(ready.get("type"), "ready")
        return worker

    def _exec(self, worker, code, rid="cell"):
        worker.stdin.write(json.dumps({"type": "execute", "id": rid, "code": code}) + "\n")
        worker.stdin.flush()
        while True:
            line = worker.stdout.readline()
            if not line:
                self.fail("kernel worker closed protocol stdout")
            response = json.loads(line)
            if response.get("type") == "result" and response.get("id") == rid:
                return response

    def _close(self, worker):
        if worker.poll() is None:
            try:
                worker.stdin.close()
            except Exception:
                pass
            try:
                self.assertEqual(worker.wait(timeout=5), 0)
            except subprocess.TimeoutExpired:
                worker.kill()
                worker.wait()
        if worker.stdin and not worker.stdin.closed:
            worker.stdin.close()
        if worker.stdout:
            worker.stdout.close()

    def _worker_abspath(self, worker, relative):
        response = self._exec(
            worker,
            f"import os; print(os.path.abspath({relative!r}))",
            rid=f"abs-{relative}",
        )
        self.assertIsNone(response.get("error"))
        return response["stdout"].strip()

    def test_linecache_keeps_only_recent_cells(self):
        worker = subprocess.Popen(
            [sys.executable, str(Path(__file__).with_name("kernel_worker.py"))],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        try:
            self.assertEqual(json.loads(worker.stdout.readline())["type"], "ready")

            for index in range(70):
                request = {
                    "type": "execute",
                    "id": str(index),
                    "code": (
                        "print(len([key for key in __import__('linecache').cache "
                        "if str(key).startswith('<wisp-kernel:')]))"
                        + "\n#"
                        + ("x" * (256 * 1024))
                    ),
                }
                worker.stdin.write(json.dumps(request) + "\n")
                worker.stdin.flush()
                while True:
                    response = json.loads(worker.stdout.readline())
                    if response.get("type") == "result" and response.get("id") == str(index):
                        break

            self.assertLessEqual(int(response["stdout"].strip()), 32)

            oversized = {
                "type": "execute",
                "id": "oversized",
                "code": "x" * (1024 * 1024 + 1),
            }
            worker.stdin.write(json.dumps(oversized) + "\n")
            worker.stdin.flush()
            response = json.loads(worker.stdout.readline())
            self.assertEqual(response["id"], "oversized")
            self.assertIn("Code exceeds", response["error"])

            too_many_lines = {
                "type": "execute",
                "id": "too-many-lines",
                "code": "pass\r" * 20_001,
            }
            worker.stdin.write(json.dumps(too_many_lines) + "\n")
            worker.stdin.flush()
            response = json.loads(worker.stdout.readline())
            self.assertEqual(response["id"], "too-many-lines")
            self.assertIn("Code exceeds", response["error"])
            worker.stdin.close()
            self.assertEqual(worker.wait(timeout=5), 0)
        finally:
            if worker.poll() is None:
                worker.kill()
                worker.wait()
            if not worker.stdin.closed:
                worker.stdin.close()
            worker.stdout.close()

    def test_computed_name_write_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "fig_1.png")
                response = self._exec(
                    worker,
                    "name = f'fig_{1}.png'\nopen(name, 'w').write('x')",
                    rid="computed",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                self.assertIn(expected, response.get("files_written") or [])
            finally:
                self._close(worker)

    @unittest.skipUnless(
        _c_level_writer_available(),
        "numpy and matplotlib are both unavailable",
    )
    def test_c_level_library_write_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                response = self._exec(
                    worker,
                    "\n".join(
                        [
                            "try:",
                            "    import numpy as np",
                            "    np.save('arr.npy', np.arange(3))",
                            "except Exception:",
                            "    import matplotlib",
                            "    matplotlib.use('Agg')",
                            "    import matplotlib.pyplot as plt",
                            "    plt.plot([1, 2, 3])",
                            "    plt.savefig('plot.png')",
                        ]
                    ),
                    rid="c-level",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                written = response.get("files_written") or []
                expected_npy = self._worker_abspath(worker, "arr.npy")
                expected_png = self._worker_abspath(worker, "plot.png")
                self.assertTrue(
                    expected_npy in written or expected_png in written,
                    written,
                )
            finally:
                self._close(worker)

    def test_append_mode_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "a.txt")
                response = self._exec(worker, "open('a.txt', 'a').write('x')", rid="append")
                self.assertIsNone(response.get("error"), response.get("error"))
                self.assertIn(expected, response.get("files_written") or [])
            finally:
                self._close(worker)

    def test_readonly_open_is_not_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, "a.txt").write_text("hi", encoding="utf-8")
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "a.txt")
                response = self._exec(worker, "open('a.txt').read()", rid="read")
                self.assertIsNone(response.get("error"), response.get("error"))
                self.assertNotIn(expected, response.get("files_written") or [])
            finally:
                self._close(worker)

    def test_failed_open_is_not_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, os.path.join("missing_dir", "nope.txt"))
                response = self._exec(
                    worker,
                    "open('missing_dir/nope.txt', 'w')",
                    rid="failed",
                )
                self.assertIsNotNone(response.get("error"))
                self.assertNotIn(expected, response.get("files_written") or [])
            finally:
                self._close(worker)

    def test_write_then_raise_still_reports_the_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "boom.txt")
                response = self._exec(
                    worker,
                    "open('boom.txt', 'w').write('x')\nraise RuntimeError('nope')",
                    rid="raise",
                )
                self.assertIsNotNone(response.get("error"))
                self.assertIn(expected, response.get("files_written") or [])
            finally:
                self._close(worker)

    def test_bytes_path_write_is_reported_and_does_not_kill_the_worker(self):
        # Regression: a bytes path used to crash json.dumps (bytes are not
        # serializable), killing the worker and the whole session's state.
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "bytes_path.txt")
                response = self._exec(
                    worker,
                    "open(b'bytes_path.txt', 'wb').write(b'x')",
                    rid="bytes",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                self.assertIn(expected, response.get("files_written") or [])
                # The worker must survive to run another cell.
                follow_up = self._exec(worker, "print('alive')", rid="bytes-2")
                self.assertIsNone(follow_up.get("error"))
            finally:
                self._close(worker)

    def test_undecodable_filename_is_skipped_not_reported(self):
        # A surrogate-escaped name (non-UTF-8 filename) must never reach the
        # protocol: the host's strict JSON parser rejects lone surrogates,
        # which would lose the whole result frame. The path is skipped and
        # falls back to host-side inference. The open itself may fail on
        # filesystems that require valid UTF-8 names (macOS); either way the
        # response must parse and must not carry the path.
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                response = self._exec(
                    worker,
                    "open('f_\\udcff.txt', 'w').write('x')",
                    rid="surrogate",
                )
                for path in response.get("files_written") or []:
                    path.encode("utf-8", "strict")
                    self.assertNotIn("f_", os.path.basename(path))
                follow_up = self._exec(worker, "print('alive')", rid="surrogate-2")
                self.assertIsNone(follow_up.get("error"))
            finally:
                self._close(worker)

    def test_sqlite3_database_write_is_not_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                expected = self._worker_abspath(worker, "db.sqlite")
                response = self._exec(
                    worker,
                    "\n".join(
                        [
                            "import sqlite3",
                            "conn = sqlite3.connect('db.sqlite')",
                            "conn.execute('create table t(x)')",
                            "conn.execute('insert into t values (1)')",
                            "conn.commit()",
                            "conn.close()",
                        ]
                    ),
                    rid="sqlite",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                written = response.get("files_written")
                self.assertIsNotNone(written)
                self.assertFalse(
                    any(
                        path == expected
                        or path.startswith(expected + "-")
                        for path in written
                    ),
                    written,
                )
            finally:
                self._close(worker)

    def test_cap_omits_files_written_when_exceeded(self):
        with tempfile.TemporaryDirectory() as tmp:
            worker = self._spawn(cwd=tmp)
            try:
                response = self._exec(
                    worker,
                    "for i in range(513):\n    open(f'f{i}.txt', 'w').write('x')",
                    rid="cap",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                self.assertNotIn("files_written", response)
            finally:
                self._close(worker)

    def test_bytecode_cache_is_not_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, "helper_mod.py").write_text("V = 1\n", encoding="utf-8")
            worker = self._spawn(cwd=tmp)
            try:
                response = self._exec(
                    worker,
                    "import sys\nsys.path.insert(0, '.')\nimport helper_mod",
                    rid="pycache",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                written = response.get("files_written")
                self.assertIsNotNone(written)
                self.assertFalse(
                    [path for path in written if "__pycache__" in path],
                    written,
                )
            finally:
                self._close(worker)

    def test_bytecode_cache_does_not_consume_the_cap(self):
        # The host discards `__pycache__` paths, so they must not evict the
        # real outputs of the same cell from the report.
        with tempfile.TemporaryDirectory() as tmp:
            for index in range(300):
                Path(tmp, f"mod_{index}.py").write_text(f"V = {index}\n", encoding="utf-8")
            worker = self._spawn(cwd=tmp)
            try:
                response = self._exec(
                    worker,
                    "\n".join(
                        [
                            "import os, sys",
                            "sys.path.insert(0, '.')",
                            "for i in range(300):",
                            "    __import__(f'mod_{i}')",
                            "os.makedirs('results', exist_ok=True)",
                            "for i in range(250):",
                            "    open(f'results/out_{i}.csv', 'w').write('x')",
                        ]
                    ),
                    rid="pycache-cap",
                )
                self.assertIsNone(response.get("error"), response.get("error"))
                written = response.get("files_written")
                self.assertIsNotNone(written, "300 discarded .pyc paths exhausted the cap")
                self.assertEqual(len(written), 250, written)
            finally:
                self._close(worker)


if __name__ == "__main__":
    unittest.main()
