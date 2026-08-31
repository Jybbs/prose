#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.14"
# ///
"""
Unit tests for the harness, one case per module, run as a script.
"""

from contextlib    import redirect_stdout
from difflib       import SequenceMatcher
from io            import StringIO
from json          import loads
from os            import environ
from pathlib       import Path
from sys           import executable
from tempfile      import TemporaryDirectory
from unittest      import TestCase, main
from unittest.mock import patch

from attribution import Attributor
from bindings    import bindings
from comparison  import MISSING, compare, divergence
from corpus      import excluded, resolve
from diff        import hunk, mapped_rows, pairing
from execute     import constant, module_name
from fixes       import drops, edit_rows, reaches, rewritten
from ratchet     import bake, baseline, judge, verdict
from records     import Break, Outcome, Width
from report      import reproduction
from runner      import Runner
from stage       import Stage, configure

CORPUS = {
    "binds.py"   : "X = 1\nNAMES = ('a', 'b')\n\n\ndef f():\n    pass\n",
    "dies.py"    : "import os, signal\nos.kill(os.getpid(), signal.SIGSEGV)\n",
    "flaky.py"   : "import time\nT = time.time_ns()\n",
    "missing.py" : "y = undefined_name\n",
    "silent.py"  : "import os\nos._exit(0)\n",
    "sleeps.py"  : "import time\ntime.sleep(60)\n"
}
MODULE = """import os.path as osp
import sys, json.decoder
from re import compile as rc, escape
from x import *

@decorated
def f(a):
    inner = 1

class K:
    attr = 2

try:
    t = 1
except ValueError:
    e = 2

match sys:
    case _:
        m = 3

squares = [n for n in range(3)]
(w := 4)
"""


class AttributionCase(TestCase):
    """
    `Attributor.locate` against a formatted tree.
    """

    def located(self, frames: tuple, module: str, name: str | None) -> tuple:
        return self.attributor.locate(
            Break(
                formatted = Outcome("raised", frames=frames),
                module    = module,
                name      = name,
                original  = Outcome("ok"),
                reason    = ""
            )
        )

    def setUp(self):
        self.scratch    = TemporaryDirectory()
        self.formatted  = written(Path(self.scratch.name), {"c.py": "Z = 1\n"})
        self.attributor = Attributor(
            binary    = "",
            covered   = {},
            formatted = self.formatted,
            label     = "default",
            runner    = None,
            width     = None
        )

    def tearDown(self):
        self.scratch.cleanup()

    def test_binding_row_stands_in_for_a_frame(self):
        self.assertEqual(
            self.located((("/usr/lib/x.py", 9),), "c.py", "Z"),
            ("c.py", 1)
        )

    def test_deepest_frame_under_the_tree_wins(self):
        frames = (
            (f"{self.formatted}/a.py", 3),
            ("/usr/lib/x.py", 9),
            (f"{self.formatted}/b.py", 7)
        )
        self.assertEqual(self.located(frames, "a.py", None), ("b.py", 7))

    def test_module_alone_where_neither_exists(self):
        self.assertEqual(self.located((), "c.py", "nope"), ("c.py", None))


class BindingsCase(TestCase):
    """
    The module-level binding walk.
    """

    def rows(self, text: str) -> dict[str, range]:

        with TemporaryDirectory() as scratch:
            return bindings(written(Path(scratch), {"m.py": text}) / "m.py")

    def test_compound_statements_are_entered_and_scopes_are_not(self):
        rows = self.rows(MODULE)

        self.assertEqual(rows["t"], range(14, 15))
        self.assertEqual(rows["e"], range(16, 17))
        self.assertEqual(rows["m"], range(20, 21))
        self.assertEqual(rows["squares"], range(22, 23))
        self.assertEqual(rows["w"], range(23, 24))

        for absent in ("inner", "attr", "n", "a", "x"):
            with self.subTest(name=absent):
                self.assertNotIn(absent, rows)

    def test_definitions_and_imports_bind_their_first_segment(self):
        rows = self.rows(MODULE)

        self.assertEqual(rows["osp"], range(1, 2))
        self.assertEqual(rows["sys"], range(2, 3))
        self.assertEqual(rows["json"], range(2, 3))
        self.assertEqual(rows["rc"], range(3, 4))
        self.assertEqual(rows["escape"], range(3, 4))
        self.assertEqual(rows["f"], range(7, 8))
        self.assertEqual(rows["K"], range(10, 11))

    def test_unparsable_module_binds_nothing(self):
        self.assertEqual(self.rows("def (\n"), {})


class ComparisonCase(TestCase):
    """
    `divergence` and `compare` over two outcomes.
    """

    def test_a_constant_no_longer_plain_reads_as_missing(self):
        original  = Outcome("ok", constants=(("N", "1"),), names=("N",))
        formatted = Outcome("ok", names=("N",))

        self.assertEqual(
            divergence(formatted, original),
            (f"binds `N` to {MISSING} where the original binds 1", "N")
        )

    def test_a_constant_rebound_names_both_values(self):
        original  = Outcome("ok", constants=(("N", "1"),), names=("N",))
        formatted = Outcome("ok", constants=(("N", "2"),), names=("N",))

        self.assertEqual(
            divergence(formatted, original),
            ("binds `N` to 2 where the original binds 1", "N")
        )

    def test_a_name_bound_only_by_the_formatted_side(self):
        self.assertEqual(
            divergence(Outcome("ok", names=("a", "z")), Outcome("ok", names=("a",))),
            ("binds `z` the original does not", "z")
        )

    def test_a_name_left_unbound(self):
        self.assertEqual(
            divergence(Outcome("ok", names=("a",)), Outcome("ok", names=("a", "b"))),
            ("leaves `b` unbound", "b")
        )

    def test_a_raised_run_returns_its_error_and_name(self):
        raised = Outcome(
            "raised",
            error = "raises NameError: name 'x' is not defined",
            name  = "x"
        )
        self.assertEqual(divergence(raised, Outcome("ok")), (raised.error, "x"))

    def test_compare_splits_breaks_comparable_and_unmeasured(self):
        before = {
            "a" : Outcome("ok", names=("x",)),
            "b" : Outcome("raised", error="e"),
            "c" : Outcome("ok")
        }
        after = {
            "a" : Outcome("ok", names=("x", "y")),
            "b" : Outcome("ok"),
            "c" : Outcome("unmeasured")
        }

        breaks, comparable, unmeasured = compare(after, before, ["a", "b", "c"])

        self.assertEqual([brk.module for brk in breaks], ["a"])
        self.assertEqual(breaks[0].reason, "binds `y` the original does not")
        self.assertEqual(breaks[0].name, "y")
        self.assertEqual(comparable, ["a"])
        self.assertEqual(unmeasured, ["c"])

    def test_identical_namespaces_do_not_diverge(self):
        same = Outcome("ok", constants=(("N", "1"),), names=("N",))
        self.assertIsNone(divergence(same, same))


class CorpusCase(TestCase):
    """
    Entry-point exclusion and target resolution.
    """

    def test_entry_points_leave_the_walk(self):

        for relative in (
            "pkg/__main__.py", "__main__.py", "test/x.py",
            "a/tests/b.py", "idlelib/idle_test/x.py", "turtledemo/x.py",
            "antigravity.py", "idlelib/idle.py", "webbrowser.py"
        ):
            with self.subTest(relative=relative):
                self.assertTrue(excluded(relative))

    def test_library_modules_stay(self):

        for relative in (
            "test_x.py", "unittest/mock.py", "a/testing/b.py", "re/_parser.py"
        ):
            with self.subTest(relative=relative):
                self.assertFalse(excluded(relative))

    def test_resolve_narrows_refuses_and_passes(self):

        with TemporaryDirectory() as scratch, TemporaryDirectory() as elsewhere:
            stdlib = written(
                root    = Path(scratch).resolve(),
                sources = {"notes.txt": "", "pkg/__init__.py": ""}
            )
            outside = written(Path(elsewhere).resolve(), {"m.py": ""})

            self.assertIsNone(resolve(stdlib, ""))
            self.assertIsNone(resolve(stdlib, str(stdlib)))
            self.assertEqual(
                resolve(stdlib, str(stdlib / "pkg/__init__.py")),
                "pkg/__init__.py"
            )

            for target in (
                outside,
                stdlib / "notes.txt",
                outside / "m.py",
                stdlib / "gone.py"
            ):
                with self.subTest(target=target), self.assertRaises(SystemExit):
                    resolve(stdlib, str(target))


class DiffCase(TestCase):
    """
    Row mapping and the hunk window over a line matcher.
    """

    def test_a_rewritten_row_maps_to_the_whole_block(self):
        pairs = matcher(["x", "Y", "Q", "z"], ["x", "y", "z"])

        self.assertEqual(mapped_rows(pairs, 2), range(2, 3))
        self.assertEqual(mapped_rows(pairs, 3), range(2, 3))
        self.assertEqual(mapped_rows(pairs, 9), range(0))

    def test_an_equal_row_maps_to_one_original_row(self):
        pairs = matcher(["x", "Y", "Q", "z"], ["x", "y", "z"])

        self.assertEqual(mapped_rows(pairs, 1), range(1, 2))
        self.assertEqual(mapped_rows(pairs, 4), range(3, 4))

    def test_an_inserted_row_maps_to_where_it_landed(self):
        self.assertEqual(
            mapped_rows(matcher(["x", "N", "z"], ["x", "z"]), 2),
            range(2, 3)
        )

    def test_an_unknown_row_lands_on_the_changed_line_naming_the_name(self):
        before = [f"l{n}" for n in range(1, 11)]
        after  = [*before[:2], "L3", *before[3:5], "L6", *before[6:]]
        lines  = hunk(matcher(after, before), None, "L6")

        self.assertIn("+L6", lines)
        self.assertNotIn("+L3", lines)
        self.assertEqual(hunk(matcher(after, before), None)[:3], [" l1", " l2", "-l3"])

    def test_pairing_reads_before_into_a_and_after_into_b(self):

        with TemporaryDirectory() as scratch:
            root  = written(Path(scratch), {"after.py": "y\n", "before.py": "x\n"})
            pairs = pairing(root / "after.py", root / "before.py")
        self.assertEqual((pairs.a, pairs.b), (["x"], ["y"]))

    def test_the_hunk_cuts_context_either_side_of_the_row(self):
        before = [f"l{n}" for n in range(1, 11)]
        after  = [*before[:5], "L6", *before[6:]]

        self.assertEqual(
            hunk(matcher(after, before), 6),
            ["...", " l4", " l5", "-l6", "+L6", " l7", " l8", " l9", "..."]
        )
        self.assertEqual(
            hunk(matcher(after, before), 1),
            [" l1", " l2", " l3", " l4", "..."]
        )


class ExecuteCase(TestCase):
    """
    The constant spelling and module naming the runner records.
    """

    def test_module_names_follow_the_import_binding(self):
        self.assertEqual(module_name("json/__init__.py"), "json")
        self.assertEqual(module_name("json/decoder.py"), "json.decoder")
        self.assertEqual(module_name("enum.py"), "enum")

    def test_plain_constants_spell_and_others_do_not(self):
        self.assertEqual(constant(None), "None")
        self.assertEqual(constant(True), "True")

        self.assertEqual(constant((1,)), "(1,)")
        self.assertEqual(constant((1, "a", (2, None))), "(1, 'a', (2, None))")
        self.assertEqual(constant(frozenset({"a", "b"})), "frozenset({'a', 'b'})")

        self.assertIsNone(constant([1]))
        self.assertIsNone(constant((1, [2])))
        self.assertIsNone(constant(1.5))


class FixesCase(TestCase):
    """
    Which rows an edit reaches and what it leaves.
    """

    def test_an_end_at_column_one_closes_on_the_row_above(self):
        self.assertEqual(edit_rows(edit("", (5, 1), (3, 1))), range(3, 5))
        self.assertEqual(edit_rows(edit("", (5, 4), (3, 1))), range(3, 6))
        self.assertEqual(edit_rows(edit("", (3, 1), (3, 1))), range(3, 4))

    def test_drops_reads_whole_words_only(self):
        edits = [edit("from m import a", (1, 19), (1, 1))]

        self.assertTrue(drops(edits, "b", "from m import a, b\n"))
        self.assertFalse(drops(edits, "a", "from m import a, b\n"))
        self.assertFalse(
            drops(
                [edit("from m import ab", (1, 17), (1, 1))],
                "a",
                "from m import ab\n"
            )
        )

    def test_reaching_by_row_overlap_or_written_line(self):
        edits = [edit("x = 1\n", (5, 1), (3, 1))]

        self.assertTrue(reaches(edits, range(4, 6)))
        self.assertFalse(reaches(edits, range(6, 7)))
        self.assertTrue(reaches(edits, range(9, 10), "x = 1"))
        self.assertFalse(reaches(edits, range(9, 10), "x = 2"))

    def test_rewritten_returns_the_reached_lines_before_and_after(self):
        text = "a = 1\nb = 2\nc = 3\n"

        self.assertEqual(
            rewritten([edit("9", (2, 6), (2, 5))], text),
            ("b = 2", "b = 9")
        )
        self.assertEqual(
            rewritten([edit("100", (1, 6), (1, 5)), edit("x", (1, 2), (1, 1))], text),
            ("a = 1", "x = 100")
        )
        self.assertEqual(
            rewritten([edit("9", (2, 6), (2, 5))], "a = 1\nb = 2"),
            ("b = 2", "b = 9")
        )


class RatchetCase(TestCase):
    """
    The baked break set and the verdict it ratchets.
    """

    def test_bake_round_trips_through_the_baseline(self):

        with TemporaryDirectory() as scratch:
            path = f"{scratch}/baseline.json"
            bake(
                path,
                [
                    width([broken("a.py", "a.py", "r1"), broken("a.py", "b.py", "r1")]),
                    width([], label="60")
                ]
            )
            with patch.dict(environ, {"PROSE_IMPORTS_BASELINE": path}):
                held = baseline()

        self.assertEqual(held, {"60": [], "default": [["a.py", "r1"]]})
        self.assertEqual(
            judge(
                width([broken("a.py", "c.py", "r1"), broken("b.py", "d.py", "r1")]),
                held
            ),
            {"c.py"}
        )

    def test_baseline_is_empty_when_unset(self):

        with patch.dict(environ, clear=True):
            self.assertEqual(baseline(), {})

    def test_verdict_bakes_and_passes(self):

        with (
            TemporaryDirectory() as scratch,
            patch.dict(environ, {"PROSE_IMPORTS_BAKE": f"{scratch}/b.json"}),
            redirect_stdout(StringIO()),
        ):
            self.assertEqual(
                verdict([(width([broken("a.py", "a.py", "r1")]), set())]),
                0
            )

            self.assertEqual(
                loads(Path(scratch, "b.json").read_text(encoding="utf-8")),
                {"default": [["a.py", "r1"]]}
            )

    def test_verdict_fails_on_a_new_break_only(self):
        found = width([broken("a.py", "a.py", "r1")])

        self.assertEqual(verdict([(found, set())]), 1)
        self.assertEqual(verdict([(found, {"a.py"})]), 0)
        self.assertEqual(verdict([(width([]), set())]), 0)

    def test_verdict_names_an_unmeasured_run_over_everything(self):

        with patch.dict(environ, {"PROSE_IMPORTS_BAKE": "/nowhere"}):
            self.assertIsInstance(
                verdict([(width([], unmeasured=["m.py"]), set())]),
                str
            )


class RecordsCase(TestCase):
    """
    The derived fields on a break and a width.
    """

    def test_a_break_falls_back_to_its_own_module_when_nothing_loaded(self):
        brk = broken("f.py", "m.py", "r")

        self.assertEqual(brk.loaded, ("m.py",))
        self.assertEqual(brk.key, ("f.py", "r"))

    def test_uncomparable_leaves_the_unmeasured_aside(self):
        found = Width(
            breaks     = [],
            candidates = 10,
            comparable = 6,
            flaky      = [],
            label      = "default",
            unmeasured = ["u.py"]
        )
        self.assertEqual(found.uncomparable, 3)


class ReportCase(TestCase):
    """
    The one-module reproduction command.
    """

    def test_knobs_are_quoted_and_the_width_appended(self):
        knob = "PROSE_IMPORTS_TIMEOUT='5 s'"
        with patch.dict(environ, {"PROSE_IMPORTS_TIMEOUT": "5 s"}, clear=True):
            self.assertEqual(
                reproduction(Path("/c"), "default", "m.py"),
                f"{knob} mise run imports /c/m.py"
            )

            self.assertEqual(
                reproduction(Path("/c"), "60", "p/q.py"),
                f"{knob} PROSE_IMPORTS_WIDTHS=60 mise run imports /c/p/q.py"
            )


class RunnerCase(TestCase):
    """
    One module run in a fresh interpreter, every outcome kind.
    """

    def original(self, module: str) -> Outcome:
        return self.runner.execute(module, [self.stage.original])

    @classmethod
    def setUpClass(cls):
        cls.scratch = TemporaryDirectory()
        cls.stage   = Stage(written(Path(cls.scratch.name), CORPUS))
        with patch.dict(environ, {"PROSE_IMPORTS_TIMEOUT": "1"}):
            cls.runner = Runner(executable, cls.stage)

    @classmethod
    def tearDownClass(cls):
        cls.runner.pool.shutdown()
        cls.scratch.cleanup()

    def test_a_binding_module_records_its_namespace(self):
        out = self.original("binds.py")

        self.assertEqual(out.kind, "ok")
        self.assertIn("X", out.names)
        self.assertIn("f", out.names)
        self.assertIn(("X", "1"), out.constants)
        self.assertIn(("NAMES", "('a', 'b')"), out.constants)
        self.assertEqual(out.loaded, ("binds.py",))

    def test_a_hang_times_out_through_the_process_group(self):
        out = self.original("sleeps.py")
        self.assertEqual((out.kind, out.error), ("timeout", "times out after 1s"))

    def test_a_raise_records_frames_and_the_missing_name(self):
        out = self.original("missing.py")

        self.assertEqual(out.kind, "raised")
        self.assertEqual(out.name, "undefined_name")
        self.assertEqual(
            out.error,
            "raises NameError: name 'undefined_name' is not defined"
        )
        self.assertEqual(out.frames[-1], (f"{self.stage.original}/missing.py", 1))

    def test_a_signal_death_is_named(self):
        out = self.original("dies.py")
        self.assertEqual((out.kind, out.error), ("raised", "dies on SIGSEGV"))

    def test_an_exit_without_a_record_is_unmeasured(self):
        out = self.original("silent.py")
        self.assertEqual((out.kind, out.error), ("unmeasured", "leaves no record"))

    def test_confirm_reports_a_flaky_original_and_holds_a_real_break(self):
        first = self.original("flaky.py")
        flaky = Break(
            formatted = first,
            module    = "flaky.py",
            name      = "T",
            original  = first,
            reason    = "r"
        )

        self.assertFalse(self.runner.confirm(flaky, self.stage.original))

        formatted = self.stage.copy("formatted")
        (formatted / "binds.py").write_text("X = 2\n", encoding="utf-8")

        after        = self.runner.execute("binds.py", [formatted])
        before       = self.original("binds.py")
        reason, name = divergence(after, before)
        real         = Break(
            formatted = after,
            module    = "binds.py",
            name      = name,
            original  = before,
            reason    = reason
        )

        self.assertTrue(self.runner.confirm(real, formatted))


class StageCase(TestCase):
    """
    Copies, overlays, and the width pin.
    """

    def test_overlay_holds_the_top_level_module_or_package_of_each_file(self):

        with TemporaryDirectory() as scratch:
            corpus = {
                "__pycache__/x.pyc" : "",
                "other.py"          : "",
                "pkg/__init__.py"   : "",
                "pkg/sub.py"        : "",
                "solo.py"           : ""
            }
            stage = Stage(written(Path(scratch), corpus))
            tree  = stage.overlay(
                ("pkg/sub.py", "solo.py"),
                "default",
                "solo.py",
                "rule-x",
                60
            )

            self.assertEqual(
                tree,
                stage.root / "alone" / "default" / "solo.py" / "rule-x"
            )

            self.assertEqual(
                sorted(
                    p.relative_to(tree).as_posix() for p in tree.rglob(
                        "*"
                    ) if p.is_file()
                ),
                ["pkg/__init__.py", "pkg/sub.py", "prose.toml", "solo.py"]
            )

            self.assertEqual(
                (tree / "prose.toml").read_text(),
                "code-line-length = 60\n"
            )
            self.assertFalse((stage.original / "__pycache__").exists())

    def test_the_default_width_writes_no_config(self):

        with TemporaryDirectory() as scratch:
            configure(Path(scratch), None)
            self.assertFalse(Path(scratch, "prose.toml").exists())


def broken(file: str, module: str, reason: str) -> Break:
    """
    Return a break of `module` at `file` for `reason`.
    """
    return Break(
        formatted = Outcome("raised", error=reason),
        frame     = (file, None),
        module    = module,
        original  = Outcome("ok"),
        name      = None,
        reason    = reason
    )


def edit(content: str, end: tuple[int, int], start: tuple[int, int]) -> dict:
    """
    Return an edit record from `start` to `end`, each a row and column,
    writing `content`.
    """
    return {
        "content"      : content,
        "end_location" : {"column": end[1], "row": end[0]},
        "location"     : {"column": start[1], "row": start[0]}
    }


def matcher(after: list[str], before: list[str]) -> SequenceMatcher:
    """
    Return the line matcher from `before` to `after`.
    """
    return SequenceMatcher(None, before, after, autojunk=False)


def width(
    breaks     : list[Break],
    label      : str              = "default",
    unmeasured : list[str] | None = None
) -> Width:
    """
    Return a width holding `breaks`, every candidate comparable unless
    `unmeasured` names one.
    """
    return Width(
        breaks     = breaks,
        candidates = len(breaks),
        comparable = len(breaks),
        flaky      = [],
        label      = label,
        unmeasured = unmeasured or []
    )


def written(root: Path, sources: dict[str, str]) -> Path:
    """
    Write each of `sources` under `root` and return `root`.
    """
    for relative, text in sources.items():
        path = root / relative
        path.parent.mkdir(exist_ok=True, parents=True)
        path.write_text(text, encoding="utf-8")
    return root


if __name__ == "__main__":

    main()
