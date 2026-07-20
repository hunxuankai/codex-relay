from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from importlib import import_module
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
TASK_SCRIPT = SCRIPTS_DIR / "task.py"
sys.path.insert(0, str(SCRIPTS_DIR))

add_session = import_module("add_session")
task_store = import_module("common.task_store")
set_active_task = import_module("common.active_task").set_active_task
get_session_commit_message = import_module(
    "common.config"
).get_session_commit_message
init_developer = import_module("common.developer").init_developer


class TaskScaffoldingTests(unittest.TestCase):
    def _create_task(self, root: Path, *, subagent: bool = False) -> Path:
        trellis_dir = root / ".trellis"
        trellis_dir.mkdir()
        (trellis_dir / ".developer").write_text("name=tester\n", encoding="utf-8")
        if subagent:
            (root / ".claude").mkdir()

        result = subprocess.run(
            [
                sys.executable,
                str(TASK_SCRIPT),
                "create",
                "中文任务",
                "--slug",
                "chinese-generated-materials",
                "--description",
                "中文目标",
                "--no-start",
            ],
            cwd=root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        task_dirs = [
            path
            for path in (trellis_dir / "tasks").iterdir()
            if path.is_dir() and path.name != "archive"
        ]
        self.assertEqual(len(task_dirs), 1)
        return task_dirs[0]

    def test_inline_task_create_writes_chinese_prd(self) -> None:
        with TemporaryDirectory() as temp_dir:
            task_dir = self._create_task(Path(temp_dir))
            prd = (task_dir / "prd.md").read_text(encoding="utf-8")

        headings = [line for line in prd.splitlines() if line.startswith("## ")]
        self.assertEqual(headings, ["## 目标", "## 需求", "## 验收标准", "## 说明"])
        self.assertIn("# 中文任务", prd)
        self.assertIn("中文目标", prd)
        self.assertIn("`prd.md`", prd)
        self.assertIn("`task.py start`", prd)
        self.assertNotIn("## Goal", prd)
        self.assertNotIn("Keep `prd.md` focused", prd)

    def test_subagent_task_create_writes_chinese_jsonl_seed(self) -> None:
        with TemporaryDirectory() as temp_dir:
            task_dir = self._create_task(Path(temp_dir), subagent=True)
            rows = []
            for name in ("implement.jsonl", "check.jsonl"):
                lines = (task_dir / name).read_text(encoding="utf-8").splitlines()
                self.assertEqual(len(lines), 1)
                rows.append(json.loads(lines[0]))

        for row in rows:
            self.assertEqual(set(row), {"_example"})
            example = row["_example"]
            self.assertRegex(example, r"[\u4e00-\u9fff]")
            self.assertIn('{"file": "<path>", "reason": "<why>"}', example)
            self.assertIn(
                "python .trellis/scripts/get_context.py --mode packages",
                example,
            )
            self.assertNotIn("Fill with", example)
            self.assertNotIn("Put spec/research files only", example)
            self.assertNotIn("Delete this line", example)


class WorkspaceGenerationTests(unittest.TestCase):
    def test_workspace_generation_updates_and_legacy_label_stay_chinese(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / ".trellis").mkdir()
            with redirect_stdout(io.StringIO()):
                self.assertTrue(init_developer("tester", root))

            workspace = root / ".trellis" / "workspace" / "tester"
            index_path = workspace / "index.md"
            journal_path = workspace / "journal-1.md"
            initial = index_path.read_text(encoding="utf-8")
            initial += journal_path.read_text(encoding="utf-8")
            self.assertIn("# 工作区索引 - tester", initial)
            self.assertIn("# 日志 - tester（第 1 部分）", initial)
            self.assertNotIn("Workspace Memory", initial)
            self.assertNotIn("# Journal -", initial)

            legacy_path = root / "legacy-index.md"
            legacy_path.write_text("- **Total Sessions**: 4\n", encoding="utf-8")
            self.assertEqual(add_session.get_current_session(legacy_path), 4)

            for session_number in (1, 2):
                with redirect_stdout(io.StringIO()):
                    self.assertTrue(
                        add_session.update_index(
                            index_path,
                            workspace,
                            f"会话 {session_number}",
                            "abc1234",
                            session_number,
                            "journal-1.md",
                            "2026-07-21",
                            "master",
                        )
                    )
                self.assertEqual(
                    add_session.get_current_session(index_path),
                    session_number,
                )

            rotated = add_session.create_new_journal_file(
                workspace,
                2,
                "tester",
                "2026-07-21",
            )
            self.assertIn(
                "# 日志 - tester（第 2 部分）",
                rotated.read_text(encoding="utf-8"),
            )


class AutomaticCommitTests(unittest.TestCase):
    def test_session_owned_task_accepts_exact_session(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            task_dir = root / ".trellis" / "tasks" / "owned-task"
            task_dir.mkdir(parents=True)
            with patch.dict(os.environ, {"TRELLIS_CONTEXT_ID": "owned-session"}):
                self.assertIsNotNone(
                    set_active_task(".trellis/tasks/owned-task", root)
                )
                self.assertEqual(
                    add_session._get_session_owned_task(root),
                    ".trellis/tasks/owned-task",
                )
                task_dir.rmdir()
                self.assertIsNone(add_session._get_session_owned_task(root))

    def test_session_commit_rejects_cross_session_fallback_task(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            task_dir = root / ".trellis" / "tasks" / "audit-task"
            sessions_dir = root / ".trellis" / ".runtime" / "sessions"
            task_dir.mkdir(parents=True)
            sessions_dir.mkdir(parents=True)
            (sessions_dir / "audit-session.json").write_text(
                json.dumps({"current_task": ".trellis/tasks/audit-task"}),
                encoding="utf-8",
            )

            selected_task_names: list[str | None] = []
            staged_paths: list[list[str]] = []

            def safe_paths(repo_root: Path, task_name: str | None = None) -> list[str]:
                self.assertEqual(repo_root, root)
                selected_task_names.append(task_name)
                return [
                    ".trellis/workspace/tester/index.md",
                    ".trellis/tasks/audit-task/prd.md",
                ]

            def safe_git_add(
                paths: list[str],
                repo_root: Path,
            ) -> tuple[bool, str, str]:
                self.assertEqual(repo_root, root)
                staged_paths.append(paths)
                return True, "", ""

            def run_git(args: list[str], cwd: Path) -> tuple[int, str, str]:
                self.assertEqual(cwd, root)
                if args[0] == "diff":
                    return 1, "", ""
                return 0, "", ""

            with (
                patch.dict(os.environ, {"TRELLIS_CONTEXT_ID": "finished-session"}),
                patch.object(add_session, "get_session_auto_commit", return_value=True),
                patch.object(
                    add_session,
                    "get_session_commit_message",
                    return_value="chore: 记录会话日志",
                ),
                patch.object(
                    add_session,
                    "safe_trellis_paths_to_add",
                    side_effect=safe_paths,
                ),
                patch.object(add_session, "safe_git_add", side_effect=safe_git_add),
                patch.object(add_session, "run_git", side_effect=run_git),
                redirect_stderr(io.StringIO()),
            ):
                add_session._auto_commit_workspace(root)

        self.assertEqual(selected_task_names, [None])
        self.assertEqual(staged_paths, [[".trellis/workspace/tester/index.md"]])

    def test_session_metadata_rejects_cross_session_fallback_task(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            task_dir = root / ".trellis" / "tasks" / "audit-task"
            sessions_dir = root / ".trellis" / ".runtime" / "sessions"
            task_dir.mkdir(parents=True)
            sessions_dir.mkdir(parents=True)
            (sessions_dir / "audit-session.json").write_text(
                json.dumps({"current_task": ".trellis/tasks/audit-task"}),
                encoding="utf-8",
            )

            with (
                patch.dict(
                    os.environ,
                    {"TRELLIS_CONTEXT_ID": "finished-session"},
                ),
                patch.object(add_session, "get_repo_root", return_value=root),
                patch.object(add_session, "load_task") as load_task_mock,
                patch.object(add_session, "resolve_package", return_value=None),
                patch.object(
                    add_session,
                    "resolve_session_branch",
                    return_value="master",
                ) as resolve_branch_mock,
                patch.object(
                    add_session,
                    "add_session",
                    return_value=0,
                ) as write_session_mock,
                patch.object(
                    sys,
                    "argv",
                    ["add_session.py", "--title", "测试会话", "--no-commit"],
                ),
            ):
                self.assertEqual(add_session.main(), 0)

        load_task_mock.assert_not_called()
        resolve_branch_mock.assert_called_once_with(root, None, None)
        write_session_mock.assert_called_once()
        self.assertIsNone(write_session_mock.call_args.kwargs["package"])
        self.assertEqual(write_session_mock.call_args.kwargs["branch"], "master")

    def test_session_commit_uses_chinese_message_and_workspace_scope(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.assertEqual(get_session_commit_message(root), "chore: 记录会话日志")
            (root / ".trellis").mkdir()
            (root / ".trellis" / "config.yaml").write_text(
                'session_commit_message: "chore: 记录会话日志"\n',
                encoding="utf-8",
            )
            self.assertEqual(get_session_commit_message(root), "chore: 记录会话日志")
            git_calls: list[list[str]] = []
            staged_paths: list[list[str]] = []

            def run_git(args: list[str], cwd: Path) -> tuple[int, str, str]:
                self.assertEqual(cwd, root)
                git_calls.append(args)
                if args[0] == "diff":
                    return 1, "", ""
                return 0, "", ""

            def safe_git_add(
                paths: list[str],
                repo_root: Path,
            ) -> tuple[bool, str, str]:
                self.assertEqual(repo_root, root)
                staged_paths.append(paths)
                return True, "", ""

            with (
                patch.object(add_session, "get_session_auto_commit", return_value=True),
                patch.object(
                    add_session,
                    "get_session_commit_message",
                    return_value="chore: 记录会话日志",
                ),
                patch.object(
                    add_session,
                    "safe_trellis_paths_to_add",
                    return_value=[
                        ".trellis/workspace/tester/index.md",
                        ".trellis/tasks/other/prd.md",
                    ],
                ),
                patch.object(add_session, "safe_git_add", side_effect=safe_git_add),
                patch.object(add_session, "run_git", side_effect=run_git),
                redirect_stderr(io.StringIO()),
            ):
                add_session._auto_commit_workspace(root)

        self.assertEqual(staged_paths, [[".trellis/workspace/tester/index.md"]])
        self.assertIn(
            ["commit", "-m", "chore: 记录会话日志"],
            git_calls,
        )

    def test_archive_commit_uses_chinese_message_and_task_scope(self) -> None:
        with TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            git_calls: list[list[str]] = []
            staged_paths: list[list[str]] = []
            archive_path = ".trellis/tasks/archive/2026-07/07-21-test"

            def run_git(args: list[str], cwd: Path) -> tuple[int, str, str]:
                self.assertEqual(cwd, root)
                git_calls.append(args)
                if args[0] == "ls-files":
                    return 0, "tracked\n", ""
                if args[0] == "diff":
                    return 1, "", ""
                return 0, "", ""

            def safe_git_add(
                paths: list[str],
                repo_root: Path,
            ) -> tuple[bool, str, str]:
                self.assertEqual(repo_root, root)
                staged_paths.append(paths)
                return True, "", ""

            with (
                patch.object(task_store, "get_session_auto_commit", return_value=True),
                patch.object(
                    task_store,
                    "safe_archive_paths_to_add",
                    return_value=[archive_path],
                ),
                patch.object(task_store, "safe_git_add", side_effect=safe_git_add),
                patch.object(task_store, "run_git", side_effect=run_git),
                redirect_stderr(io.StringIO()),
            ):
                self.assertTrue(task_store._auto_commit_archive("07-21-test", root))

        self.assertEqual(staged_paths, [[archive_path]])
        self.assertIn(
            ["commit", "-m", "chore(task): 归档 07-21-test"],
            git_calls,
        )


if __name__ == "__main__":
    unittest.main()
