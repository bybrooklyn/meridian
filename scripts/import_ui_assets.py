#!/usr/bin/env python3
"""Import and verify Meridian UI 1.0 font/icon assets.

Generator: scripts/import_ui_assets.py
Input schema: meridian.ui-assets/v1
Version: 1
Regenerate: python3 scripts/import_ui_assets.py --fetch

Network access occurs only with ``--fetch``. Normal invocation verifies the
checked-in immutable subset against pinned SHA-256 digests.
"""

from __future__ import annotations

import argparse
import hashlib
import io
from pathlib import Path
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile


ROOT = Path(__file__).resolve().parents[1]
READ_CHUNK_BYTES = 64 * 1024
ALLOWED_FETCH_HOSTS = {"github.com", "raw.githubusercontent.com"}
ALLOWED_REDIRECT_HOST_SUFFIXES = (".githubusercontent.com",)


class AssetError(RuntimeError):
    """Raised when an immutable UI asset differs from its reviewed source."""


ARCHIVES = (
    {
        "name": "Mona Sans v2.0.27",
        "url": "https://github.com/github/mona-sans/releases/download/v2.0.27/mona-sans-variable-v2.0.27.zip",
        "bytes": 2_674_251,
        "sha256": "a95127550b2957ff84cd636d4532b227ddc33d3485082437fa27816ef1d066ec",
        "members": (
            (
                "fonts/variable/MonaSansVF[opsz,wght].ttf",
                "engine/meridian_ui_text/assets/fonts/MonaSansVF.ttf",
                "84aae10d4427a1947e96b1fd9b26c3109ffa0f50f2faae8ce460ca1e34889ed5",
                347_676,
            ),
            (
                "OFL.txt",
                "third_party/licenses/mona-sans-OFL-1.1.txt",
                "9261dcb61fb5e3c587d50d7a9fdae12bc7422d8822d7ac06b8f34550479575de",
                4_419,
            ),
        ),
    },
    {
        "name": "Hubot Sans v1.0.1",
        "url": "https://github.com/github/hubot-sans/releases/download/v1.0.1/Hubot-Sans.zip",
        "bytes": 5_363_947,
        "sha256": "b460d36097a5c9a3e45710cbe1554589eaa5765d7c2c88df364516f3e27159b1",
        "members": (
            (
                "Hubot Sans/Hubot-Sans.ttf",
                "engine/meridian_ui_text/assets/fonts/HubotSansVF.ttf",
                "2cbf834f750ae1201a8d6193b004584bd530bbad7907e02991d4a97da44784ce",
                355_100,
            ),
            (
                "Hubot Sans/LICENSE",
                "third_party/licenses/hubot-sans-OFL-1.1.txt",
                "399d10fc3ea083de0c53b6999e39750ac2e42947d50e0c566d59cc32fb5ab3b6",
                4_404,
            ),
        ),
    },
    {
        "name": "JetBrains Mono v2.304",
        "url": "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip",
        "bytes": 5_622_857,
        "sha256": "6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf",
        "members": (
            (
                "fonts/variable/JetBrainsMono[wght].ttf",
                "engine/meridian_ui_text/assets/fonts/JetBrainsMonoVF.ttf",
                "662a196d58f1183bf2d77428b6d5283fe3f45161ab021bea4036bc98e5cac016",
                303_144,
            ),
            (
                "OFL.txt",
                "third_party/licenses/jetbrains-mono-OFL-1.1.txt",
                "30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad",
                4_399,
            ),
        ),
    },
    {
        "name": "Lucide 1.25.0",
        "url": "https://github.com/lucide-icons/lucide/releases/download/1.25.0/lucide-icons-1.25.0.zip",
        "bytes": 1_292_865,
        "sha256": "070ca0b59b5b9c6587f9d09a033d8085596938784b71cfb3da9837c02b2b3a71",
        "members": tuple(
            (
                f"icons/{name}.svg",
                f"engine/meridian_ui_render/assets/icons/{name}.svg",
                digest,
                byte_count,
            )
            for name, digest, byte_count in (
                ("play", "d7c34786135922a92b6896f6c2384ceeb0346afbf6041dc79982011411409833", 306),
                ("square", "bd979354f0ab184b95cecf03eedefe40c2dc65830ac6d7e60017b2b25a354acb", 261),
                ("hammer", "db75caf31bd080726be0c2dab09372498b999cbdcf10c756f44636831f9529f5", 483),
                ("search", "283d371c2e433817bb9c0c8310caa6c77fa4177c0f4f1168d9c83b97af7389dc", 275),
                ("settings", "0ae27fd0f81999229e3127ac96c5b32edfea448e291d509e76212b917551d66b", 586),
                ("ellipsis", "4f495cc72013ffdfec677f03b33a150f7b4dd741979283fd6853a09024bca112", 312),
                ("x", "4a9cdab38fbb96162e7dace28e33f4ca0e49d8963a6162abc3d4691b7d675117", 260),
                ("chevron-down", "66ea878e72ed3488bb3b464c39dfdccee8d1f78e560dccea40e5e12da0e87e87", 236),
                ("chevron-right", "2758143d7b2434e4aa7307dfd34405c87909ff4052f21b5f3f40d45224b4f19b", 237),
                ("triangle-alert", "4866f38b8560d410f21e3226413e0b77997b6dfbb6931fadfe0a0d5aef9ffeb4", 345),
                ("circle-x", "bcd8788901e6f29e1b231a81ba5e707d083d06cb4848a28f29407fab4f8e0b64", 293),
                ("circle-check", "3e519680ab8e2a8ad8f56a340c10d61957d872237aaa868cf324b0900a74f384", 273),
            )
        ),
    },
)

LUCIDE_LICENSE = {
    "name": "Lucide 1.25.0 license",
    "url": "https://raw.githubusercontent.com/lucide-icons/lucide/1.25.0/LICENSE",
    "bytes": 3_208,
    "destination": "third_party/licenses/lucide-ISC-MIT.txt",
    "sha256": "b495047bd93a9b06913511076f504daba17d5bbeb3e0650f3bb53a4220329c57",
}


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_reviewed(source, expected_bytes: int, label: str) -> bytes:
    chunks = []
    total = 0
    while True:
        chunk = source.read(min(READ_CHUNK_BYTES, expected_bytes + 1 - total))
        if not chunk:
            break
        total += len(chunk)
        if total > expected_bytes:
            raise AssetError(f"{label} exceeds reviewed size {expected_bytes} bytes")
        chunks.append(chunk)
    if total != expected_bytes:
        raise AssetError(f"{label} has {total} bytes; reviewed size is {expected_bytes}")
    return b"".join(chunks)


def reviewed_host(hostname: str | None) -> bool:
    return bool(
        hostname
        and (
            hostname in ALLOWED_FETCH_HOSTS
            or any(hostname.endswith(suffix) for suffix in ALLOWED_REDIRECT_HOST_SUFFIXES)
        )
    )


def checked_asset_url(url: str, *, allow_redirect_host: bool) -> urllib.parse.ParseResult:
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme != "https" or (
        parsed.hostname not in ALLOWED_FETCH_HOSTS
        and not (allow_redirect_host and reviewed_host(parsed.hostname))
    ):
        raise AssetError(f"{url} is not an approved HTTPS asset source")
    return parsed


class ReviewedRedirectHandler(urllib.request.HTTPRedirectHandler):
    """Reject redirects before urllib opens an unreviewed hop."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        checked_asset_url(req.full_url, allow_redirect_host=True)
        checked_asset_url(urllib.parse.urljoin(req.full_url, newurl), allow_redirect_host=True)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


OPENER = urllib.request.build_opener(ReviewedRedirectHandler)


def reviewed_url(url: str) -> urllib.request.Request:
    checked_asset_url(url, allow_redirect_host=False)
    return urllib.request.Request(url, headers={"User-Agent": "Meridian-UI-asset-import/1"})


def download(url: str, expected_bytes: int) -> bytes:
    request = reviewed_url(url)
    try:
        response = OPENER.open(request, timeout=60)
    except urllib.error.URLError as error:
        raise AssetError(f"{url} fetch failed: {error}") from error
    with response:
        final_url = response.geturl()
        checked_asset_url(final_url, allow_redirect_host=True)
        declared = response.headers.get("Content-Length")
        if declared is not None:
            try:
                declared_bytes = int(declared)
            except ValueError as error:
                raise AssetError(f"{url} returned malformed Content-Length {declared!r}") from error
            if declared_bytes != expected_bytes:
                raise AssetError(
                    f"{url} declares {declared_bytes} bytes; reviewed size is {expected_bytes}"
                )
        return read_reviewed(response, expected_bytes, url)


def checked(payload: bytes, expected: str, label: str) -> bytes:
    actual = digest(payload)
    if actual != expected:
        raise AssetError(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")
    return payload


def write(destination: str, payload: bytes) -> None:
    path = checked_destination(destination)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)


def checked_destination(destination: str) -> Path:
    path = (ROOT / destination).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError as error:
        raise AssetError(f"{destination} escapes the repository root") from error
    return path


def checked_member_info(source: zipfile.ZipFile, member: str, expected_bytes: int) -> zipfile.ZipInfo:
    if member.startswith("/") or "\\" in member or ".." in Path(member).parts:
        raise AssetError(f"{member} is not a reviewed relative archive path")
    matches = [info for info in source.infolist() if info.filename == member]
    if len(matches) != 1:
        raise AssetError(f"{member} appears {len(matches)} times in the reviewed archive")
    info = matches[0]
    if info.is_dir():
        raise AssetError(f"{member} is a directory, not a reviewed file")
    if info.file_size != expected_bytes:
        raise AssetError(
            f"{member} expands to {info.file_size} bytes; reviewed size is {expected_bytes}"
        )
    return info


def fetch() -> None:
    for archive in ARCHIVES:
        payload = checked(
            download(archive["url"], archive["bytes"]),
            archive["sha256"],
            archive["name"],
        )
        with zipfile.ZipFile(io.BytesIO(payload)) as source:
            for member, destination, expected, expected_bytes in archive["members"]:
                info = checked_member_info(source, member, expected_bytes)
                with source.open(info) as entry:
                    contents = read_reviewed(entry, expected_bytes, member)
                write(destination, checked(contents, expected, member))
    write(
        LUCIDE_LICENSE["destination"],
        checked(
            download(LUCIDE_LICENSE["url"], LUCIDE_LICENSE["bytes"]),
            LUCIDE_LICENSE["sha256"],
            LUCIDE_LICENSE["name"],
        ),
    )


def verify() -> None:
    expected_files = [
        (member[1], member[2], member[3])
        for archive in ARCHIVES
        for member in archive["members"]
    ]
    expected_files.append(
        (LUCIDE_LICENSE["destination"], LUCIDE_LICENSE["sha256"], LUCIDE_LICENSE["bytes"])
    )
    for destination, expected, expected_bytes in expected_files:
        path = checked_destination(destination)
        if not path.is_file():
            raise AssetError(f"missing UI asset: {destination}")
        actual_bytes = path.stat().st_size
        if actual_bytes != expected_bytes:
            raise AssetError(
                f"{destination} has {actual_bytes} bytes; reviewed size is {expected_bytes}"
            )
        checked(path.read_bytes(), expected, destination)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fetch", action="store_true", help="download pinned reviewed assets")
    args = parser.parse_args()
    try:
        if args.fetch:
            fetch()
        verify()
    except (AssetError, OSError, zipfile.BadZipFile) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("Meridian UI assets verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
