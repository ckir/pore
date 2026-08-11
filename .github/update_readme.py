#!/usr/bin/env python
import os
import os.path
import re
import subprocess
from typing import List

HERE = os.path.dirname(__file__)
ROOT = os.path.abspath(os.path.join(HERE, os.path.pardir))
README = os.path.join(ROOT, "README.md")


def replace_section(file: str, start_pat: str, end_pat: str, lines: List[str]) -> None:
    prefix_lines: List[str] = []
    postfix_lines: List[str] = []
    file_lines = prefix_lines
    found_section = False
    with open(file, "r", encoding="utf-8") as ifile:
        inside_section = False
        for line in ifile:
            if inside_section:
                if re.match(end_pat, line):
                    inside_section = False
                    file_lines = postfix_lines
                    file_lines.append(line)
            else:
                if re.match(start_pat, line):
                    inside_section = True
                    found_section = True
                file_lines.append(line)

    if inside_section or not found_section:
        raise Exception(f"could not find file section {start_pat}")

    all_lines = prefix_lines + lines + postfix_lines
    with open(file, "w", encoding="utf-8") as ofile:
        ofile.write("".join(all_lines))


def get_help_lines(args: str) -> List[str]:
    """Run `cargo run -- <args>` and return the help output lines.

    Returns all lines starting from and including the 'Usage:' line.
    """
    output = subprocess.getoutput(f"cargo run -- {args}")
    lines = output.splitlines()
    # Find the Usage: line (clap v4 uses "Usage:", older versions used "USAGE:")
    i = None
    for idx, line in enumerate(lines):
        if line.startswith("Usage:") or line == "USAGE:":
            i = idx
            break
    if i is None:
        raise ValueError(f"Could not find 'Usage:' line in '{args}' output")
    return [l.rstrip().replace("pore.exe", "pore") + "\n" for l in lines[i:]]


def main() -> None:
    """Update the README with usage for all subcommands."""
    top_lines = get_help_lines("--help")
    search_lines = get_help_lines("search --help")
    eval_lines = get_help_lines("eval --help")

    all_lines = []
    # Top-level help (skip the first "Usage:" line since replace_section
    # preserves the start-pattern line from the README)
    all_lines.extend(top_lines[1:])
    all_lines.append("\n")
    all_lines.append("### `pore search`\n")
    all_lines.append("\n")
    all_lines.extend(search_lines)
    all_lines.append("\n")
    all_lines.append("### `pore eval`\n")
    all_lines.append("\n")
    all_lines.extend(eval_lines)

    replace_section(README, r"^Usage", r"^```$", all_lines)



if __name__ == "__main__":
    main()
