#!/usr/bin/env python3
"""Draft converter: AUTOSAR YAML templates -> ARXML.

Usage:
    python tools/yaml2arxml/yaml2arxml.py templates/autosar/system.yaml -o out.arxml
"""
import argparse
import sys
import yaml


def build_arxml(data):
    autosar = data.get("autosar", {})
    system_name = autosar.get("system_name", "System")
    ns = "http://autosar.org/schema/r4.0"

    lines = []
    lines.append('<?xml version="1.0" encoding="UTF-8"?>')
    lines.append(f'<AUTOSAR xmlns="{ns}">')
    lines.append("  <AR-PACKAGES>")
    lines.append(f"    <AR-PACKAGE>")
    lines.append(f"      <SHORT-NAME>{system_name}</SHORT-NAME>")
    lines.append("      <ELEMENTS>")

    for comp in autosar.get("components", []):
        lines.append(f"        <APPLICATION-SW-COMPONENT-TYPE>")
        lines.append(f"          <SHORT-NAME>{comp['name']}</SHORT-NAME>")
        lines.append("        </APPLICATION-SW-COMPONENT-TYPE>")

    lines.append("      </ELEMENTS>")
    lines.append("    </AR-PACKAGE>")
    lines.append("  </AR-PACKAGES>")
    lines.append("</AUTOSAR>")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input", help="input YAML file")
    parser.add_argument("-o", "--output", help="output ARXML file")
    args = parser.parse_args()

    with open(args.input, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)

    xml = build_arxml(data)
    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(xml)
        print(f"wrote {args.output}")
    else:
        sys.stdout.write(xml)


if __name__ == "__main__":
    main()