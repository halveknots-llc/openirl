from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "static_validate.py"
SPEC = importlib.util.spec_from_file_location("openirl_static_validate", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load static validation module")
static_validate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(static_validate)


class ActionsPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="openirl-static-policy-")
        self.root = Path(self.temporary.name)
        (self.root / ".github" / "workflows").mkdir(parents=True)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write(self, relative: str, content: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_accepts_recursive_local_graph_with_immutable_external_refs(self) -> None:
        commit = "a" * 40
        digest = "b" * 64
        self.write(
            ".github/workflows/ci.yml",
            f"""
name: ci
jobs:
  reuse:
    uses: ./.github/workflows/reuse.yml
  local:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/openirl/checks@sha256:{digest}
    steps:
      - uses: ./.github/actions/local
""",
        )
        self.write(
            ".github/workflows/reuse.yml",
            f"""
name: reuse
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@{commit}
""",
        )
        self.write(
            ".github/actions/local/action.yml",
            f"""
name: local
runs:
  using: composite
  steps:
    - uses: docker://ghcr.io/openirl/checks@sha256:{digest}
""",
        )

        self.assertEqual(static_validate.validate_actions_policy(self.root), [])

    def test_rejects_mutable_external_ref_even_when_yaml_is_folded(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses : >-
          actions/checkout@v4
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "action-pin" for _, kind, _ in findings))

    def test_ignores_an_input_named_uses(self) -> None:
        commit = "a" * 40
        self.write(
            ".github/workflows/ci.yml",
            f"""
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@{commit}
        with:
          uses: a domain-specific input value
""",
        )

        self.assertEqual(static_validate.validate_actions_policy(self.root), [])

    def test_rejects_mutable_docker_image_in_local_action(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/local
""",
        )
        self.write(
            ".github/actions/local/action.yaml",
            """
name: local
runs:
  using: docker
  image: docker://alpine:3.22
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "container-pin" for _, kind, _ in findings))

    def test_rejects_mutable_transitive_composite_dependency(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/local
""",
        )
        self.write(
            ".github/actions/local/action.yml",
            """
name: local
runs:
  using: composite
  steps:
    - uses: actions/setup-python@v6
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "action-pin" for _, kind, _ in findings))

    def test_rejects_local_action_cycle(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/actions/first
""",
        )
        self.write(
            ".github/actions/first/action.yml",
            """
name: first
runs:
  using: composite
  steps:
    - uses: ./.github/actions/second
""",
        )
        self.write(
            ".github/actions/second/action.yml",
            """
name: second
runs:
  using: composite
  steps:
    - uses: ./.github/actions/first
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "local-action-cycle" for _, kind, _ in findings))

    def test_rejects_local_reference_that_escapes_repository(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: ./../outside
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "local-action" for _, kind, _ in findings))


class PublicEvidencePolicyTests(unittest.TestCase):
    def test_detects_a_guarded_surface_that_drops_required_language(self) -> None:
        with tempfile.TemporaryDirectory(prefix="openirl-public-evidence-") as temporary:
            root = Path(temporary)
            (root / "docs").mkdir()
            policy = {
                "schema_version": 1,
                "policy_id": "test-policy",
                "documentation": "docs/review.md",
                "required_review_terms": ["stream credentials", "local paths"],
                "surfaces": ["docs/review.md", "docs/report.md"],
            }
            (root / "docs" / "public-evidence-policy.json").write_text(
                json.dumps(policy), encoding="utf-8"
            )
            (root / "docs" / "review.md").write_text(
                "stream credentials and local paths", encoding="utf-8"
            )
            (root / "docs" / "report.md").write_text(
                "stream credentials", encoding="utf-8"
            )

            findings = static_validate.validate_public_evidence_policy(root)
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0][0], "docs/report.md")
            self.assertIn("local paths", findings[0][2])


if __name__ == "__main__":
    unittest.main()
