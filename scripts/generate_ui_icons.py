#!/usr/bin/env python3
"""Generate bounded Meridian path geometry from pinned Lucide SVG sources.

Generator: scripts/generate_ui_icons.py
Input schema: meridian.ui-icons/v1
Version: 1
Regenerate: python3 scripts/generate_ui_icons.py --write
"""

from __future__ import annotations

import argparse
import hashlib
import math
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
ICON_ROOT = ROOT / "engine/meridian_ui_render/assets/icons"
OUTPUT = ROOT / "engine/meridian_ui_render/src/generated_icons.rs"
TOKEN = re.compile(r"[A-Za-z]|[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?")
MAX_PATH_COMMANDS = 4_096
ICON_COORD_MIN = 0.0
ICON_COORD_MAX = 24.0
ICON_COORD_EPSILON = 0.000001
ICONS = (
    ("play", "Play", "d7c34786135922a92b6896f6c2384ceeb0346afbf6041dc79982011411409833"),
    ("square", "Stop", "bd979354f0ab184b95cecf03eedefe40c2dc65830ac6d7e60017b2b25a354acb"),
    ("hammer", "Build", "db75caf31bd080726be0c2dab09372498b999cbdcf10c756f44636831f9529f5"),
    ("search", "Search", "283d371c2e433817bb9c0c8310caa6c77fa4177c0f4f1168d9c83b97af7389dc"),
    ("settings", "Settings", "0ae27fd0f81999229e3127ac96c5b32edfea448e291d509e76212b917551d66b"),
    ("ellipsis", "More", "4f495cc72013ffdfec677f03b33a150f7b4dd741979283fd6853a09024bca112"),
    ("x", "Close", "4a9cdab38fbb96162e7dace28e33f4ca0e49d8963a6162abc3d4691b7d675117"),
    ("chevron-down", "ChevronDown", "66ea878e72ed3488bb3b464c39dfdccee8d1f78e560dccea40e5e12da0e87e87"),
    ("chevron-right", "ChevronRight", "2758143d7b2434e4aa7307dfd34405c87909ff4052f21b5f3f40d45224b4f19b"),
    ("triangle-alert", "Warning", "4866f38b8560d410f21e3226413e0b77997b6dfbb6931fadfe0a0d5aef9ffeb4"),
    ("circle-x", "Error", "bcd8788901e6f29e1b231a81ba5e707d083d06cb4848a28f29407fab4f8e0b64"),
    ("circle-check", "Success", "3e519680ab8e2a8ad8f56a340c10d61957d872237aaa868cf324b0900a74f384"),
)


class IconError(RuntimeError):
    """Raised when reviewed icon source cannot lower deterministically."""


def checked_number(value, label):
    value = float(value)
    if not math.isfinite(value):
        raise IconError(f"{label} is not finite")
    return value


def checked_coordinate(value, label):
    value = checked_number(value, label)
    if value < ICON_COORD_MIN - ICON_COORD_EPSILON or value > ICON_COORD_MAX + ICON_COORD_EPSILON:
        raise IconError(
            f"{label}={value} is outside reviewed Lucide 24x24 bounds "
            f"{ICON_COORD_MIN:g}..{ICON_COORD_MAX:g}"
        )
    return value


def checked_dimension(value, label):
    value = checked_coordinate(value, label)
    if value < 0.0:
        raise IconError(f"{label}={value} is negative")
    return value


def checked_point(point, label):
    return (
        checked_coordinate(point[0], f"{label}.x"),
        checked_coordinate(point[1], f"{label}.y"),
    )


def arc_points(start, rx, ry, rotation_degrees, large_arc, sweep, end):
    if start == end or rx == 0.0 or ry == 0.0:
        return [checked_point(end, "arc.end")]
    rx, ry = abs(rx), abs(ry)
    rotation = math.radians(rotation_degrees % 360.0)
    cos_rotation, sin_rotation = math.cos(rotation), math.sin(rotation)
    dx, dy = (start[0] - end[0]) / 2.0, (start[1] - end[1]) / 2.0
    local_x = cos_rotation * dx + sin_rotation * dy
    local_y = -sin_rotation * dx + cos_rotation * dy
    scale = local_x * local_x / (rx * rx) + local_y * local_y / (ry * ry)
    if scale > 1.0:
        scale = math.sqrt(scale)
        rx, ry = rx * scale, ry * scale
    numerator = max(0.0, rx * rx * ry * ry - rx * rx * local_y * local_y - ry * ry * local_x * local_x)
    denominator = rx * rx * local_y * local_y + ry * ry * local_x * local_x
    coefficient = 0.0 if denominator == 0.0 else math.sqrt(numerator / denominator)
    if large_arc == sweep:
        coefficient = -coefficient
    center_local_x = coefficient * rx * local_y / ry
    center_local_y = coefficient * -ry * local_x / rx
    center_x = cos_rotation * center_local_x - sin_rotation * center_local_y + (start[0] + end[0]) / 2.0
    center_y = sin_rotation * center_local_x + cos_rotation * center_local_y + (start[1] + end[1]) / 2.0

    def signed_angle(a, b):
        length = math.hypot(*a) * math.hypot(*b)
        if length == 0.0:
            return 0.0
        value = math.acos(max(-1.0, min(1.0, (a[0] * b[0] + a[1] * b[1]) / length)))
        return -value if a[0] * b[1] - a[1] * b[0] < 0.0 else value

    start_vector = ((local_x - center_local_x) / rx, (local_y - center_local_y) / ry)
    end_vector = ((-local_x - center_local_x) / rx, (-local_y - center_local_y) / ry)
    start_angle = signed_angle((1.0, 0.0), start_vector)
    delta = signed_angle(start_vector, end_vector)
    if not sweep and delta > 0.0:
        delta -= math.tau
    elif sweep and delta < 0.0:
        delta += math.tau
    segment_count = max(1, min(32, math.ceil(abs(delta) / (math.pi / 8.0))))
    points = []
    for segment in range(1, segment_count + 1):
        angle = start_angle + delta * segment / segment_count
        ellipse_x, ellipse_y = rx * math.cos(angle), ry * math.sin(angle)
        points.append(checked_point((
            cos_rotation * ellipse_x - sin_rotation * ellipse_y + center_x,
            sin_rotation * ellipse_x + cos_rotation * ellipse_y + center_y,
        ), "arc.point"))
    points[-1] = end
    return points


def parse_path(data):
    tokens = tokenize_path(data)
    commands, index, active = [], 0, ""
    cursor = (0.0, 0.0)
    subpath = cursor

    def number():
        nonlocal index
        if index >= len(tokens) or tokens[index].isalpha():
            raise IconError(f"malformed path near token {index}: {data}")
        value = checked_number(tokens[index], f"SVG number near token {index}")
        index += 1
        return value

    def flag():
        value = number()
        if value not in (0.0, 1.0):
            raise IconError(f"SVG arc flags must be 0 or 1: {data}")
        return bool(int(value))

    def target(relative):
        x, y = number(), number()
        point = (cursor[0] + x, cursor[1] + y) if relative else (x, y)
        return checked_point(point, f"SVG path target near token {index}")

    while index < len(tokens):
        if tokens[index].isalpha():
            active = tokens[index]
            index += 1
        if not active:
            raise IconError(f"path starts without command: {data}")
        kind, relative = active.upper(), active.islower()
        if kind == "Z":
            commands.append(("Close",))
            cursor, active = subpath, ""
        elif kind == "M":
            cursor = target(relative)
            commands.append(("MoveTo", *cursor))
            subpath = cursor
            active = "l" if relative else "L"
        elif kind == "L":
            cursor = target(relative)
            commands.append(("LineTo", *cursor))
        elif kind == "H":
            cursor = checked_point(
                (number() + (cursor[0] if relative else 0.0), cursor[1]),
                f"SVG horizontal target near token {index}",
            )
            commands.append(("LineTo", *cursor))
        elif kind == "V":
            cursor = checked_point(
                (cursor[0], number() + (cursor[1] if relative else 0.0)),
                f"SVG vertical target near token {index}",
            )
            commands.append(("LineTo", *cursor))
        elif kind == "A":
            rx, ry, rotation = number(), number(), number()
            if rx < 0.0 or ry < 0.0 or rx > ICON_COORD_MAX or ry > ICON_COORD_MAX:
                raise IconError(f"SVG arc radii must fit reviewed 24x24 bounds: {data}")
            large_arc, sweep = flag(), flag()
            end = target(relative)
            for value in arc_points(cursor, rx, ry, rotation, large_arc, sweep, end):
                commands.append(("LineTo", *value))
            cursor = end
        else:
            raise IconError(f"unsupported SVG command {active!r}")
        if len(commands) > MAX_PATH_COMMANDS:
            raise IconError(f"path exceeds {MAX_PATH_COMMANDS} lowered commands: {data}")
    return commands


def tokenize_path(data):
    tokens = []
    end = 0
    for match in TOKEN.finditer(data):
        if data[end:match.start()].replace(",", "").strip():
            raise IconError(f"unsupported SVG path syntax near {data[end:match.start()]!r}")
        tokens.append(match.group(0))
        end = match.end()
    if data[end:].replace(",", "").strip():
        raise IconError(f"unsupported SVG path syntax near {data[end:]!r}")
    return tokens


def circle(cx, cy, radius):
    cx = checked_coordinate(cx, "circle.cx")
    cy = checked_coordinate(cy, "circle.cy")
    radius = checked_dimension(radius, "circle.r")
    checked_coordinate(cx - radius, "circle.left")
    checked_coordinate(cx + radius, "circle.right")
    checked_coordinate(cy - radius, "circle.top")
    checked_coordinate(cy + radius, "circle.bottom")
    control = radius * 0.5522847498307936
    return [
        ("MoveTo", cx + radius, cy),
        ("CubicTo", cx + radius, cy + control, cx + control, cy + radius, cx, cy + radius),
        ("CubicTo", cx - control, cy + radius, cx - radius, cy + control, cx - radius, cy),
        ("CubicTo", cx - radius, cy - control, cx - control, cy - radius, cx, cy - radius),
        ("CubicTo", cx + control, cy - radius, cx + radius, cy - control, cx + radius, cy),
        ("Close",),
    ]


def rectangle(x, y, width, height, radius):
    x = checked_coordinate(x, "rect.x")
    y = checked_coordinate(y, "rect.y")
    width = checked_dimension(width, "rect.width")
    height = checked_dimension(height, "rect.height")
    radius = checked_dimension(radius, "rect.rx")
    checked_coordinate(x + width, "rect.right")
    checked_coordinate(y + height, "rect.bottom")
    radius = max(0.0, min(radius, width / 2.0, height / 2.0))
    return [
        ("MoveTo", x + radius, y),
        ("LineTo", x + width - radius, y),
        ("QuadraticTo", x + width, y, x + width, y + radius),
        ("LineTo", x + width, y + height - radius),
        ("QuadraticTo", x + width, y + height, x + width - radius, y + height),
        ("LineTo", x + radius, y + height),
        ("QuadraticTo", x, y + height, x, y + height - radius),
        ("LineTo", x, y + radius),
        ("QuadraticTo", x, y, x + radius, y),
        ("Close",),
    ]


def shapes(path):
    root = ET.fromstring(path.read_text(encoding="utf-8"))
    if root.tag.rsplit("}", 1)[-1] != "svg":
        raise IconError(f"{path.name} root is not <svg>")
    if root.text and root.text.strip():
        raise IconError(f"{path.name} contains unexpected SVG root text")
    expected_root = {
        "width": "24",
        "height": "24",
        "viewBox": "0 0 24 24",
        "fill": "none",
        "stroke": "currentColor",
        "stroke-width": "2",
        "stroke-linecap": "round",
        "stroke-linejoin": "round",
    }
    for attribute, expected in expected_root.items():
        if root.attrib.get(attribute) != expected:
            raise IconError(
                f"{path.name} has unsupported SVG {attribute}={root.attrib.get(attribute)!r}"
            )
    if set(root.attrib) != set(expected_root):
        raise IconError(f"{path.name} contains unexpected SVG root attributes")
    result = []
    for element in root:
        if element.text and element.text.strip():
            raise IconError(f"{path.name} contains unexpected SVG child text")
        if element.tail and element.tail.strip():
            raise IconError(f"{path.name} contains unexpected SVG child tail")
        tag = element.tag.rsplit("}", 1)[-1]
        unsupported = {
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "style",
            "class",
        }.intersection(element.attrib)
        if unsupported:
            raise IconError(f"{path.name} contains per-shape SVG paint/style attributes")
        if tag == "path":
            if set(element.attrib) != {"d"}:
                raise IconError(f"{path.name} contains unexpected path attributes")
            result.append(parse_path(element.attrib["d"]))
        elif tag == "circle":
            if set(element.attrib) != {"cx", "cy", "r"}:
                raise IconError(f"{path.name} contains unexpected circle attributes")
            result.append(circle(float(element.attrib["cx"]), float(element.attrib["cy"]), float(element.attrib["r"])))
        elif tag == "rect":
            if not {"width", "height"}.issubset(element.attrib) or not set(
                element.attrib
            ).issubset({"x", "y", "width", "height", "rx"}):
                raise IconError(f"{path.name} contains unexpected rect attributes")
            result.append(rectangle(float(element.attrib.get("x", 0)), float(element.attrib.get("y", 0)), float(element.attrib["width"]), float(element.attrib["height"]), float(element.attrib.get("rx", 0))))
        else:
            raise IconError(f"unsupported <{tag}> in {path.name}")
    return result


def rust_float(value):
    value = 0.0 if abs(value) < 0.0000005 else round(value, 6)
    text = f"{value:.6f}".rstrip("0").rstrip(".")
    return text if "." in text else text + ".0"


def rust_command(command):
    kind, values = command[0], command[1:]
    if kind in {"MoveTo", "LineTo"}:
        return f"UiPathCommand::{kind}(UiPoint {{ x: {rust_float(values[0])}, y: {rust_float(values[1])} }})"
    if kind == "QuadraticTo":
        return f"UiPathCommand::QuadraticTo {{ control: UiPoint {{ x: {rust_float(values[0])}, y: {rust_float(values[1])} }}, end: UiPoint {{ x: {rust_float(values[2])}, y: {rust_float(values[3])} }} }}"
    if kind == "CubicTo":
        return f"UiPathCommand::CubicTo {{ control_a: UiPoint {{ x: {rust_float(values[0])}, y: {rust_float(values[1])} }}, control_b: UiPoint {{ x: {rust_float(values[2])}, y: {rust_float(values[3])} }}, end: UiPoint {{ x: {rust_float(values[4])}, y: {rust_float(values[5])} }} }}"
    return "UiPathCommand::Close"


def generate():
    lines = [
        "// @generated by scripts/generate_ui_icons.py",
        "// Input schema: meridian.ui-icons/v1; generator version: 1",
        "// Regenerate: python3 scripts/generate_ui_icons.py --write",
        "",
    ]
    variants = []
    for source_name, variant, expected_source_hash in ICONS:
        source = ICON_ROOT / f"{source_name}.svg"
        source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
        if source_hash != expected_source_hash:
            raise IconError(
                f"{source.name} SHA-256 mismatch: expected {expected_source_hash}, got {source_hash}"
            )
        lines.append(f"// {source_name}.svg SHA-256 {source_hash}")
        names = []
        for index, commands in enumerate(shapes(source)):
            name = f"ICON_{variant.upper()}_{index}"
            names.append(name)
            lines.append(f"static {name}: &[UiPathCommand] = &[")
            lines.extend(f"    {rust_command(command)}," for command in commands)
            lines.append("];")
        group = f"ICON_{variant.upper()}"
        variants.append((variant, group))
        lines.append(f"static {group}: &[&[UiPathCommand]] = &[")
        lines.extend(f"    {name}," for name in names)
        lines.extend(["];", ""])
    lines.extend([
        "pub(crate) const fn source_icon_paths(icon: IconId) -> &'static [&'static [UiPathCommand]] {",
        "    match icon {",
    ])
    lines.extend(f"        IconId::{variant} => {group}," for variant, group in variants)
    lines.extend(["    }", "}", ""])
    return "\n".join(lines).encode("utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.write == args.check:
        parser.error("choose exactly one of --write or --check")
    try:
        expected = generate()
        if args.write:
            OUTPUT.write_bytes(expected)
        elif not OUTPUT.is_file() or OUTPUT.read_bytes() != expected:
            raise IconError("generated icon geometry is stale; run with --write")
    except (IconError, OSError, ET.ParseError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("Meridian UI icon geometry verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
