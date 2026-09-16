"""Validate the source contracts, including formats and negative examples.

Run with the dependencies in scripts/requirements.txt. Never dereference the web.
"""
import json
import re
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker
from openapi_spec_validator import validate

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / "openspec/changes/bootstrap-rove/api"
documents = {}
operations = set()
refs = examples = 0


def walk(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


def validator(document, schema):
    return Draft202012Validator(
        {**schema, "components": document["components"]},
        format_checker=FormatChecker(),
    )


for path in sorted(API.glob("*.openapi.json")):
    document = documents[path.name] = json.loads(path.read_text())
    validate(document)
    for node in walk(document):
        if "$ref" in node:
            ref = node["$ref"]
            assert ref.startswith("#/"), f"External reference: {ref}"
            target = document
            for part in ref[2:].split("/"):
                target = target[part.replace("~1", "/").replace("~0", "~")]
            refs += 1
        if "operationId" in node:
            name = node["operationId"]
            assert name not in operations, f"Duplicate operation: {name}"
            operations.add(name)
    for name, schema in document["components"]["schemas"].items():
        Draft202012Validator.check_schema(schema)
        check = validator(document, schema)
        for example in schema.get("examples", []):
            check.validate(example)
            examples += 1

cases = json.loads((API / "contract-examples.json").read_text())["cases"]
for case in cases:
    document = documents[case["api"]]
    check = validator(document, document["components"]["schemas"][case["schema"]])
    assert check.is_valid(case["value"]) == case["valid"], case["schema"]

assert len(operations) == 36, "Update acceptance mapping when the API changes"
parity = json.loads((ROOT / "docs/ui-parity.json").read_text())
agent_operations = {node["operationId"] for node in walk(documents["rove-agent.openapi.json"]) if "operationId" in node}
assert set(parity["operations"]) == agent_operations, "Synchronize GUI/CLI operation matrix"
for entry in parity["operations"].values():
    assert entry["cli"] and entry["gui"]
    source = (ROOT / entry["source"]).resolve()
    assert source.is_relative_to(ROOT) and source.is_file()
mapping = json.loads((ROOT / "docs/acceptance-map.json").read_text())
requirements = (API.parent / "requirements.md").read_text()
stories = (API.parent / "user-stories.md").read_text()
assert set(mapping["requirements"]) == set(re.findall(r"^### (FR-\d+)", requirements, re.M))
assert set(mapping["quality"]) == set(re.findall(r"\| (NFR-\d+)", requirements))
mapped_operations, mapped_stories, mapped_specs = set(), set(), set()
for entry in mapping["requirements"].values():
    mapped_operations.update(entry["operations"])
    mapped_stories.update(entry["stories"])
    mapped_specs.update(entry["specs"])
    assert entry["status"] in {"partial", "verified_local", "not_verified"}
    assert entry["remaining"], "Acceptance must state its remaining platform scope"
    for evidence in entry["evidence"]:
        path = (ROOT / evidence).resolve()
        assert path.is_relative_to(ROOT) and path.is_file(), evidence
assert mapped_operations == operations, "Synchronize endpoint acceptance mapping"
assert mapped_stories == set(re.findall(r"^## (US-\d+)", stories, re.M))
assert mapped_specs == {p.name for p in (API.parent / "specs").iterdir() if p.is_dir()}
for entry in mapping["quality"].values():
    assert set(entry["requirements"]) <= set(mapping["requirements"])
    assert entry["status"] in {"partial", "verified_local", "not_verified"}
print(f"PASS: {len(documents)} OpenAPI documents, {len(operations)} operations, "
      f"{refs} references, {examples} examples, {len(cases)} positive/negative cases; "
      f"11 FR, 9 NFR, {len(mapped_stories)} stories, {len(mapped_specs)} specs mapped")
