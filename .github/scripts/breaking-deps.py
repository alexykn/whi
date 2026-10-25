"""Report direct dependencies that moved to a semver-incompatible version.

Usage: breaking-deps.py OLD_CARGO_LOCK NEW_CARGO_LOCK

Compares two Cargo.lock files. A direct dependency is one the workspace
members (packages without a registry source) depend on. Following Cargo's
rules, versions are compatible when they share the leftmost non-zero
component, and pre-releases are compatible only with themselves. Prints each
breaking change and exits 1 when there is one, 0 otherwise.
"""

import sys
from collections import defaultdict

import tomllib


def compatibility_key(version: str) -> tuple:
    version = version.split("+")[0]
    if "-" in version:
        return ("pre", version)
    major, minor, patch = (int(part) for part in version.split("."))
    if major:
        return (major,)
    if minor:
        return (0, minor)
    return (0, 0, patch)


def direct_versions(lock_path: str) -> dict[str, set[str]]:
    with open(lock_path, "rb") as lock:
        packages = tomllib.load(lock).get("package", [])
    versions = defaultdict(set)
    for package in packages:
        versions[package["name"]].add(package["version"])
    members = [package for package in packages if "source" not in package]
    member_names = {package["name"] for package in members}
    direct = defaultdict(set)
    for member in members:
        for dependency in member.get("dependencies", []):
            name, _, version = dependency.partition(" ")
            if name in member_names:
                continue
            direct[name].update({version} if version else versions[name])
    return direct


def breaking_changes(old_lock: str, new_lock: str) -> list[str]:
    old, new = direct_versions(old_lock), direct_versions(new_lock)
    changes = []
    for name in sorted(new.keys() & old.keys()):
        old_keys = {compatibility_key(version) for version in old[name]}
        for version in sorted(new[name] - old[name]):
            if compatibility_key(version) not in old_keys:
                previous = ", ".join(sorted(old[name]))
                changes.append(f"{name} {previous} -> {version}")
    return changes


def main() -> int:
    changes = breaking_changes(sys.argv[1], sys.argv[2])
    for change in changes:
        print(change)
    return 1 if changes else 0


if __name__ == "__main__":
    sys.exit(main())
