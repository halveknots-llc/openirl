#!/usr/bin/env python3
"""Static validation for the OpenIRL source repository."""
from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path, PurePosixPath
from typing import Any, Iterator

try:
    import yaml
except ImportError:  # Reported as a validation finding with install guidance.
    yaml = None

ROOT = Path(__file__).resolve().parents[1]
TEXT_SUFFIXES = {
    ".rs",
    ".md",
    ".toml",
    ".yml",
    ".yaml",
    ".json",
    ".sh",
    ".ps1",
    ".py",
    ".html",
    ".txt",
    ".conf",
}
MAX_POLICY_FILE_BYTES = 2 * 1024 * 1024
CONTAINER_DIGEST = re.compile(r"[^@\s]+@sha256:[0-9a-f]{64}")
EXTERNAL_ACTION = re.compile(
    r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.\-/]+)?@[0-9a-f]{40}"
)
UNTRUSTED_RUN_CONTEXT = re.compile(r"\$\{\{\s*github\.(?:ref|ref_name)\s*}}")
TRUSTED_PUSH_EVENT = re.compile(r"github\.event_name\s*==\s*['\"]push['\"]")
TRUSTED_RELEASE_REF = re.compile(
    r"(?:github\.ref\s*==\s*['\"]refs/heads/main['\"]|"
    r"startsWith\(\s*github\.ref\s*,\s*['\"]refs/tags/v['\"]\s*\))"
)

Finding = tuple[str, str, str]


def joined(parts: list[str]) -> str:
    return "".join(parts)


DENIED_TERMS = [
    joined(["T", "O", "D", "O"]),
    joined(["F", "I", "X", "M", "E"]),
    joined(["X", "X", "X"]),
    joined(["s", "t", "u", "b"]),
    joined(["p", "l", "a", "c", "e", "h", "o", "l", "d", "e", "r"]),
    joined(["n", "o", "t", " ", "i", "m", "p", "l", "e", "m", "e", "n", "t", "e", "d"]),
    joined(["d", "r", "y", "-", "r", "u", "n", "-", "o", "n", "l", "y"]),
    joined(["R", "e", "a", "d", "y", "F", "o", "r", "S", "m", "o", "k", "e"]),
    joined(
        [
            "N",
            "e",
            "e",
            "d",
            "s",
            "L",
            "i",
            "v",
            "e",
            "V",
            "a",
            "l",
            "i",
            "d",
            "a",
            "t",
            "i",
            "o",
            "n",
        ]
    ),
]
LEGACY_TERMS = [
    joined(["I", "M", "P", "L", "E", "M", "E", "N", "T", "A", "T", "I", "O", "N", "_", "W", "A", "V", "E"]),
    joined(["P", "R", "E", "_", "C", "O", "D", "E", "X", "_", "W", "A", "V", "E"]),
    joined(["p", "r", "e", "_", "c", "o", "d", "e", "x", "_", "w", "a", "v", "e", "s"]),
    joined(["I", "T", "E", "R", "-"]),
]
LEGACY_WORD = joined(["w", "a", "v", "e"])


def files(root: Path = ROOT) -> Iterator[Path]:
    for path in root.rglob("*"):
        if path.name.startswith("._"):
            continue
        if (
            path.is_file()
            and path.suffix.lower() in TEXT_SUFFIXES
            and "target" not in path.parts
            and ".git" not in path.parts
        ):
            yield path


def rel(path: Path, root: Path = ROOT) -> str:
    return str(path.resolve().relative_to(root.resolve())).replace("\\", "/")


def load_yaml_document(path: Path, root: Path, findings: list[Finding]) -> Any | None:
    if yaml is None:
        findings.append(
            (
                "requirements/static-validation.txt",
                "actions-policy",
                "PyYAML is required; install the pinned static-validation requirements",
            )
        )
        return None
    try:
        if path.stat().st_size > MAX_POLICY_FILE_BYTES:
            raise ValueError("workflow policy file exceeds 2 MiB")
        return yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as exc:
        findings.append((rel(path, root), "actions-yaml", str(exc)))
        return None


def repository_relative_path(root: Path, reference: str) -> Path:
    if not reference.startswith("./") or "\\" in reference or "\0" in reference:
        raise ValueError("local action reference must use a ./ repository-relative POSIX path")
    raw = reference[2:]
    parts = raw.split("/")
    if not raw or any(part in ("", ".", "..") for part in parts):
        raise ValueError("local action reference contains an unsafe path segment")
    relative = PurePosixPath(raw)
    candidate = (root / Path(*relative.parts)).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as exc:
        raise ValueError("local action reference escapes the repository") from exc
    return candidate


def validate_container_image(location: str, value: Any, findings: list[Finding]) -> None:
    image = value.get("image") if isinstance(value, dict) else value
    if not isinstance(image, str) or CONTAINER_DIGEST.fullmatch(image.strip()) is None:
        findings.append(
            (
                location,
                "container-pin",
                "workflow container images must use an immutable sha256 digest",
            )
        )


def validate_workflow_containers(location: str, document: Any, findings: list[Finding]) -> None:
    if not isinstance(document, dict):
        return
    jobs = document.get("jobs")
    if not isinstance(jobs, dict):
        return
    for job in jobs.values():
        if not isinstance(job, dict):
            continue
        if "container" in job:
            validate_container_image(location, job["container"], findings)
        services = job.get("services")
        if isinstance(services, dict):
            for service in services.values():
                validate_container_image(location, service, findings)


def permission_is_write(permissions: Any, name: str) -> bool:
    if permissions == "write-all":
        return True
    return isinstance(permissions, dict) and permissions.get(name) == "write"


def validate_workflow_privilege_boundaries(
    location: str, document: Any, findings: list[Finding]
) -> None:
    if not isinstance(document, dict):
        return
    jobs = document.get("jobs")
    if not isinstance(jobs, dict):
        return
    workflow_permissions = document.get("permissions")
    for job_name, job in jobs.items():
        if not isinstance(job, dict):
            continue
        condition = job.get("if", "")
        condition_text = condition if isinstance(condition, str) else ""
        permissions = job.get("permissions", workflow_permissions)
        privileged = any(
            permission_is_write(permissions, name)
            for name in ("attestations", "contents", "id-token")
        )
        if privileged and (
            TRUSTED_PUSH_EVENT.search(condition_text) is None
            or TRUSTED_RELEASE_REF.search(condition_text) is None
        ):
            findings.append(
                (
                    location,
                    "privileged-job-condition",
                    f"job {job_name} must require a trusted push ref before write permissions",
                )
            )

        steps = job.get("steps")
        if not isinstance(steps, list):
            continue
        for index, step in enumerate(steps, 1):
            if not isinstance(step, dict):
                continue
            run = step.get("run")
            if isinstance(run, str) and UNTRUSTED_RUN_CONTEXT.search(run):
                findings.append(
                    (
                        location,
                        "run-context-boundary",
                        f"job {job_name} step {index} must pass ref context through env",
                    )
                )


def action_references(document: dict[str, Any], document_kind: str) -> list[Any]:
    references: list[Any] = []
    if document_kind == "workflow":
        jobs = document.get("jobs")
        if not isinstance(jobs, dict):
            return references
        containers = jobs.values()
    else:
        runs = document.get("runs")
        containers = [runs] if isinstance(runs, dict) else []
    for container in containers:
        if not isinstance(container, dict):
            continue
        if document_kind == "workflow" and "uses" in container:
            references.append(container["uses"])
        steps = container.get("steps")
        if not isinstance(steps, list):
            continue
        for step in steps:
            if isinstance(step, dict) and "uses" in step:
                references.append(step["uses"])
    return references


def validate_actions_policy(root: Path = ROOT) -> list[Finding]:
    if yaml is None:
        return [
            (
                "requirements/static-validation.txt",
                "actions-policy",
                "PyYAML is required; install the pinned static-validation requirements",
            )
        ]
    findings: list[Finding] = []
    scanned: set[Path] = set()
    active: set[Path] = set()

    def scan_document(path: Path, document_kind: str) -> None:
        resolved = path.resolve()
        try:
            location = resolved.relative_to(root.resolve()).as_posix()
        except ValueError:
            findings.append(
                (
                    "local action reference",
                    "local-action",
                    "referenced policy file resolves outside the repository",
                )
            )
            return
        if resolved in active:
            findings.append((location, "local-action-cycle", "local action references form a cycle"))
            return
        if resolved in scanned:
            return
        if not path.is_file():
            findings.append((rel(path, root), "local-action", "referenced policy file is missing"))
            return
        active.add(resolved)
        try:
            document = load_yaml_document(path, root, findings)
            if document is None:
                return
            if not isinstance(document, dict):
                findings.append((location, "actions-yaml", "top-level YAML value must be a mapping"))
                return
            if document_kind == "workflow":
                validate_workflow_containers(location, document, findings)
                validate_workflow_privilege_boundaries(location, document, findings)
            elif document_kind == "action":
                runs = document.get("runs")
                if isinstance(runs, dict):
                    for image_key in ("image", "pre-entrypoint", "post-entrypoint"):
                        image = runs.get(image_key)
                        if isinstance(image, str) and image.startswith("docker://"):
                            validate_container_image(location, image.removeprefix("docker://"), findings)
            for raw_reference in action_references(document, document_kind):
                if not isinstance(raw_reference, str):
                    findings.append((location, "action-pin", "uses value must be a string"))
                    continue
                reference = raw_reference.strip()
                if reference.startswith("docker://"):
                    image = reference.removeprefix("docker://")
                    validate_container_image(location, image, findings)
                elif reference.startswith("./"):
                    try:
                        target = repository_relative_path(root, reference)
                    except ValueError as exc:
                        findings.append((location, "local-action", str(exc)))
                        continue
                    workflow_root = (root / ".github" / "workflows").resolve()
                    is_workflow = target.suffix.lower() in {".yml", ".yaml"}
                    if is_workflow:
                        try:
                            target.relative_to(workflow_root)
                        except ValueError:
                            findings.append(
                                (
                                    location,
                                    "local-workflow",
                                    "local reusable workflows must live under .github/workflows",
                                )
                            )
                            continue
                        scan_document(target, "workflow")
                        continue
                    if not target.is_dir():
                        findings.append((location, "local-action", f"{reference} is not an action directory"))
                        continue
                    metadata = next(
                        (candidate for candidate in (target / "action.yml", target / "action.yaml") if candidate.is_file()),
                        None,
                    )
                    if metadata is None:
                        findings.append((location, "local-action", f"{reference} has no action.yml or action.yaml"))
                        continue
                    scan_document(metadata, "action")
                elif EXTERNAL_ACTION.fullmatch(reference) is None:
                    findings.append(
                        (
                            location,
                            "action-pin",
                            f"{reference} must use an immutable 40-character commit",
                        )
                    )
        finally:
            active.remove(resolved)
            scanned.add(resolved)

    workflow_root = root / ".github" / "workflows"
    if not workflow_root.is_dir():
        return [(".github/workflows", "actions-policy", "workflow directory is missing")]
    for pattern in ("*.yml", "*.yaml"):
        for workflow in sorted(workflow_root.glob(pattern)):
            scan_document(workflow, "workflow")
    return findings


def validate_public_evidence_policy(root: Path = ROOT) -> list[Finding]:
    findings: list[Finding] = []
    policy_path = root / "docs" / "public-evidence-policy.json"
    try:
        policy = json.loads(policy_path.read_text(encoding="utf-8"))
    except Exception as exc:
        return [(rel(policy_path, root), "public-evidence-policy", str(exc))]

    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        return [
            (
                rel(policy_path, root),
                "public-evidence-policy",
                "policy must be a schema_version 1 mapping",
            )
        ]
    terms = policy.get("required_review_terms")
    surfaces = policy.get("surfaces")
    if (
        not isinstance(terms, list)
        or not terms
        or any(not isinstance(term, str) or not term.strip() for term in terms)
        or len(set(terms)) != len(terms)
    ):
        findings.append((rel(policy_path, root), "public-evidence-policy", "review terms must be unique strings"))
        return findings
    if (
        not isinstance(surfaces, list)
        or not surfaces
        or any(not isinstance(surface, str) or not surface.strip() for surface in surfaces)
        or len(set(surfaces)) != len(surfaces)
    ):
        findings.append((rel(policy_path, root), "public-evidence-policy", "surfaces must be unique paths"))
        return findings
    documentation = policy.get("documentation")
    if not isinstance(documentation, str) or documentation not in surfaces:
        findings.append(
            (
                rel(policy_path, root),
                "public-evidence-policy",
                "documentation must name one of the guarded surfaces",
            )
        )

    for surface in surfaces:
        try:
            candidate = repository_relative_path(root, f"./{surface}")
        except ValueError as exc:
            findings.append((surface, "public-evidence-policy", str(exc)))
            continue
        if not candidate.is_file():
            findings.append((surface, "public-evidence-policy", "guarded surface is missing"))
            continue
        if candidate.stat().st_size > MAX_POLICY_FILE_BYTES:
            findings.append((surface, "public-evidence-policy", "guarded surface exceeds 2 MiB"))
            continue
        text = re.sub(r"\s+", " ", candidate.read_text(encoding="utf-8")).casefold()
        for term in terms:
            if term.casefold() not in text:
                findings.append(
                    (
                        surface,
                        "public-evidence-policy",
                        f"missing required review term: {term}",
                    )
                )
    return findings


def main() -> int:
    findings: list[Finding] = []
    findings.extend(validate_actions_policy())
    findings.extend(validate_public_evidence_policy())

    for path in ROOT.rglob("*.json"):
        if path.name.startswith("._") or "target" in path.parts:
            continue
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:
            findings.append((rel(path), "json", str(exc)))
    for path in ROOT.rglob("*.toml"):
        if path.name.startswith("._") or "target" in path.parts:
            continue
        try:
            tomllib.loads(path.read_text(encoding="utf-8"))
        except Exception as exc:
            findings.append((rel(path), "toml", str(exc)))

    denied = [re.compile(re.escape(term), re.I) for term in DENIED_TERMS + LEGACY_TERMS]
    legacy_word = re.compile(re.escape(LEGACY_WORD), re.I)
    for path in files():
        relative = rel(path)
        if legacy_word.search(relative):
            findings.append((relative, "filename", "legacy numbered-pass label in filename"))
        text = path.read_text(encoding="utf-8", errors="replace")
        for idx, line in enumerate(text.splitlines(), 1):
            if "Redacted password value sample" in line:
                continue
            if legacy_word.search(line):
                findings.append((f"{relative}:{idx}", "legacy-label", line.strip()[:160]))
                continue
            for pattern in denied:
                if pattern.search(line):
                    findings.append((f"{relative}:{idx}", "marker", line.strip()[:160]))
                    break

    required = [
        "README.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "Cargo.toml",
        "apps/openirl-agent/src/main.rs",
        "crates/openirl-v1/src/lib.rs",
        "docs/ARCHITECTURE.md",
        "docs/SECURITY.md",
        "docs/README.md",
        "docs/VALIDATION.md",
        "docs/MAINTAINER_CHECKS.md",
        "docs/COMPATIBILITY.md",
        "docs/PUBLIC_EVIDENCE.md",
        "docs/RELEASE_PROVENANCE.md",
        "docs/public-evidence-policy.json",
        "requirements/static-validation.txt",
        "compatibility/matrix-v1.json",
        "fixtures/field/compatibility-evidence.template.json",
        "docs/features/obs-reconciliation.md",
        "scripts/audit/handoff_audit.py",
        "scripts/security/release-artifact-scan.py",
        "scripts/tests/test_static_validate.py",
        "scripts/windows/verify-alpha-portable.ps1",
        ".github/workflows/windows-package.yml",
        "release/ALPHA_RELEASE_NOTES.md",
        "docs/contributing/README.md",
        "docs/contributing/encoder-profile.md",
        "docs/contributing/relay-backend.md",
        "docs/contributing/metrics-parser.md",
        "docs/contributing/redaction-canary.md",
        "docs/contributing/live-smoke.md",
        "fixtures/contributing/encoder-profile.sample.json",
        "fixtures/contributing/relay-process.sample.json",
        "fixtures/contributing/metrics-exporter.sample.prom",
        "fixtures/contributing/redaction-canary.sample.json",
        "fixtures/contributing/live-smoke-evidence.sample.json",
        "issue_templates/feature_request.md",
    ]
    for item in required:
        if not (ROOT / item).exists():
            findings.append((item, "inventory", "missing required file"))
    if findings:
        print("static validation: fail")
        for location, kind, message in findings[:100]:
            print(f"[{kind}] {location}: {message}")
        if len(findings) > 100:
            print(f"... {len(findings) - 100} additional findings")
        return 1
    print("static validation: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
