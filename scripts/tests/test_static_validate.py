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

    def test_rejects_ref_context_embedded_in_run_script(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - run: echo "${{ github.ref_name }}"
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(any(kind == "run-context-boundary" for _, kind, _ in findings))

    def test_rejects_write_permissions_without_trusted_push_ref(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  attest:
    if: github.event_name != 'pull_request'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
      attestations: write
    steps:
      - run: echo checked
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(
            any(kind == "privileged-job-condition" for _, kind, _ in findings)
        )

    def test_rejects_negative_main_ref_condition_for_write_permissions(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  attest:
    if: github.event_name == 'push' && github.ref != 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - run: echo checked
""",
        )

        findings = static_validate.validate_actions_policy(self.root)
        self.assertTrue(
            any(kind == "privileged-job-condition" for _, kind, _ in findings)
        )

    def test_accepts_trusted_push_ref_and_env_bound_context(self) -> None:
        self.write(
            ".github/workflows/ci.yml",
            """
name: ci
jobs:
  attest:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
      attestations: write
    steps:
      - env:
          RELEASE_REF: ${{ github.ref_name }}
        run: echo "${RELEASE_REF}"
""",
        )

        self.assertEqual(static_validate.validate_actions_policy(self.root), [])


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


class ReleaseVerificationDocsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="openirl-release-trust-")
        self.root = Path(self.temporary.name)
        (self.root / "docs" / "runbooks").mkdir(parents=True)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_guides(self, content: str) -> None:
        for relative in static_validate.RELEASE_VERIFICATION_DOCS:
            (self.root / relative).write_text(content, encoding="utf-8")

    def test_accepts_independent_tag_revision_and_supplemental_check(self) -> None:
        self.write_guides(
            """
$releaseAuthenticated = $true
$revision = gh api "repos/$repo/commits/$tag" --jq '.sha'
Local verification supplements release authentication.
"""
        )

        self.assertEqual(
            static_validate.validate_release_verification_docs(self.root), []
        )

    def test_rejects_manifest_derived_expected_revision(self) -> None:
        self.write_guides(
            """
$releaseAuthenticated = $true
$revision = gh api "repos/$repo/commits/$tag" --jq '.sha'
Local verification supplements release authentication.
$expected = (Get-Content -Raw package.manifest.json | ConvertFrom-Json).source_revision
"""
        )

        findings = static_validate.validate_release_verification_docs(self.root)
        self.assertEqual(len(findings), 2)
        self.assertTrue(
            all(kind == "release-trust-policy" for _, kind, _ in findings)
        )


class GitAttributesTests(unittest.TestCase):
    def test_accepts_cross_platform_lf_policy(self) -> None:
        with tempfile.TemporaryDirectory(prefix="openirl-git-attributes-") as temporary:
            root = Path(temporary)
            (root / ".gitattributes").write_text(
                "# stable source\n*  text=auto   eol=lf\n", encoding="utf-8"
            )

            self.assertEqual(static_validate.validate_git_attributes(root), [])

    def test_rejects_platform_dependent_text_checkout(self) -> None:
        with tempfile.TemporaryDirectory(prefix="openirl-git-attributes-") as temporary:
            root = Path(temporary)
            (root / ".gitattributes").write_text("* text=auto\n", encoding="utf-8")

            findings = static_validate.validate_git_attributes(root)
            self.assertEqual(
                findings,
                [
                    (
                        ".gitattributes",
                        "line-ending-policy",
                        "text files must use LF in every checkout",
                    )
                ],
            )


class LocalMediaTemplateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="openirl-media-policy-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.documents = {}
        for relative in static_validate.LOCAL_MEDIAMTX_TEMPLATES:
            source = static_validate.ROOT / relative
            self.documents[relative] = static_validate.yaml.safe_load(source.read_text())
            self.write(relative, self.documents[relative])

    def write(self, relative: str, document: dict) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(static_validate.yaml.safe_dump(document), encoding="utf-8")

    def test_shipped_templates_explicitly_restrict_every_listener(self) -> None:
        self.assertEqual(static_validate.validate_local_mediamtx_templates(self.root), [])

    def test_omitting_upstream_defaults_is_rejected(self) -> None:
        for relative, original in self.documents.items():
            for key in ("rtsp", "hls", "moq", "webrtcAddress", "webrtcLocalUDPAddress"):
                with self.subTest(template=relative, omitted=key):
                    document = dict(original)
                    del document[key]
                    self.write(relative, document)
                    self.assertTrue(static_validate.validate_local_mediamtx_templates(self.root))
                    self.write(relative, original)

    def test_public_media_and_ice_exposure_is_rejected(self) -> None:
        relative = static_validate.LOCAL_MEDIAMTX_TEMPLATES[0]
        original = self.documents[relative]
        mutations = [
            (key, "0.0.0.0:8889")
            for key in ("apiAddress", "metricsAddress", "rtmpAddress", "srtAddress", "webrtcAddress", "webrtcLocalUDPAddress")
        ] + [
            ("webrtcLocalTCPAddress", ":8189"),
            ("webrtcIPsFromInterfaces", True),
            ("webrtcAdditionalHosts", ["media.example"]),
            ("webrtcICEServers2", [{"url": "stun:stun.example:3478"}]),
            ("webrtcAllowOrigins", ["*"]),
            ("webrtcAllowOrigins", ["http://untrusted.example"]),
            ("moq", True),
        ]
        for key, value in mutations:
            with self.subTest(key=key, value=value):
                self.write(relative, {**original, key: value})
                self.assertTrue(static_validate.validate_local_mediamtx_templates(self.root))

    def test_missing_or_malformed_template_fails_closed(self) -> None:
        path = self.root / static_validate.LOCAL_MEDIAMTX_TEMPLATES[0]
        for value in ("[]", "webrtc: [", "null"):
            path.write_text(value, encoding="utf-8")
            self.assertTrue(static_validate.validate_local_mediamtx_templates(self.root))
        path.unlink()
        self.assertTrue(static_validate.validate_local_mediamtx_templates(self.root))

    def test_socket_address_requires_literal_loopback_and_valid_port(self) -> None:
        for value in ("127.0.0.1:8189", "[::1]:8189"):
            self.assertTrue(static_validate.loopback_socket_address(value))
        for value in (None, ":8189", "0.0.0.0:8189", "[::]:8189", "localhost:8189", "127.0.0.1:0", "127.0.0.1:65536", "[[::1]]:8189"):
            self.assertFalse(static_validate.loopback_socket_address(value))


if __name__ == "__main__":
    unittest.main()
