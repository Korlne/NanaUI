#!/usr/bin/env python3
"""List the functions of a built binary whose stack frame is over a budget.

Reads each function's prologue in the disassembly and adds what it takes off
the stack pointer: the immediate `sub` forms and, for frames past a page, the
probed size (`sub x9, sp, #N` on aarch64; `mov eax, N` before
`__rust_probestack` / `__chkstk` on x86_64). The frame of a function is what
every call to it costs, whether or not it recurses; see
`docs/reference/resource-budgets.md`.

    cargo test --release -p nana-ui-scene --test deep_tree_stack --no-run
    python3 scripts/audit-stack-frames.py target/release/deps/deep_tree_stack-<hash> \
        --budget 16384 --crates nana_

Exit status is 1 when a function of a listed crate is over `--budget`.
"""

import argparse
import re
import shutil
import subprocess
import sys

SYMBOL = re.compile(r"^[0-9a-f]+ <(.+)>:$")
IMMEDIATE = r"#(0x[0-9a-f]+|\d+)(, lsl #12)?"


def immediate(match):
    value = int(match.group(1), 0)
    return value << 12 if match.group(2) else value


def frames(lines):
    """Yields (symbol, bytes) for every function in an objdump listing."""
    symbol, total, seen, probed, registers = None, 0, 0, False, {}
    for line in lines:
        header = SYMBOL.match(line.strip())
        if header:
            if symbol is not None:
                yield symbol, total
            symbol, total, seen, probed, registers = header.group(1), 0, 0, False, {}
            continue
        if symbol is None or "\t" not in line:
            continue
        seen += 1
        if seen > 40:
            continue
        # Drop the address and objdump's trailing `; =0x5000` comment.
        instruction = " ".join(line.split("\t")[1:]).split(";")[0].strip()
        # aarch64
        if m := re.match(r"sub\s+x9, sp, " + IMMEDIATE, instruction):
            total += immediate(m)
            probed = True
            continue
        if m := re.match(r"sub\s+sp, sp, " + IMMEDIATE, instruction):
            if probed and m.group(2) and int(m.group(1), 0) == 1:
                # The probe loop's page step; the probe already counted it.
                probed = False
                continue
            total += immediate(m)
            continue
        if m := re.match(r"movk?\s+[wx](9|15|16), #(0x[0-9a-f]+|\d+)(, lsl #(\d+))?", instruction):
            shift = int(m.group(4)) if m.group(4) else 0
            value = int(m.group(2), 0) << shift
            registers[m.group(1)] = (
                registers.get(m.group(1), 0) | value if instruction.startswith("movk") else value
            )
            continue
        if m := re.match(r"sub\s+sp, sp, x(9|15|16)", instruction):
            total += registers.get(m.group(1), 0)
            continue
        # x86_64 (AT&T or Intel syntax)
        if m := re.match(r"subq?\s+\$(0x[0-9a-f]+|\d+), %rsp", instruction) or re.match(
            r"sub\s+rsp, (0x[0-9a-f]+|\d+)", instruction
        ):
            total += int(m.group(1), 0)
            continue
        if m := re.match(r"movl?\s+\$(0x[0-9a-f]+|\d+), %eax", instruction) or re.match(
            r"mov\s+eax, (0x[0-9a-f]+|\d+)", instruction
        ):
            registers["eax"] = int(m.group(1), 0)
            continue
        if "probestack" in instruction or "chkstk" in instruction:
            total += registers.get("eax", 0)
    if symbol is not None:
        yield symbol, total


def demangle(symbols):
    """Readable names with `rustfilt` (`cargo install rustfilt`); raw otherwise."""
    tool = shutil.which("rustfilt")
    if tool is None:
        return symbols
    names = [s[1:] if s.startswith("__") else s for s in symbols]
    out = subprocess.run([tool], input="\n".join(names), capture_output=True, text=True)
    return out.stdout.split("\n")[: len(symbols)]


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("binary")
    parser.add_argument("--budget", type=int, default=16 << 10, help="bytes (default 16 KiB)")
    parser.add_argument(
        "--crates",
        default="nana",
        help="only gate symbols containing this text (default: nana)",
    )
    parser.add_argument("--top", type=int, default=30)
    args = parser.parse_args()

    listing = subprocess.run(
        ["objdump", "-d", "--no-show-raw-insn", args.binary],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()
    largest = {}
    for symbol, size in frames(listing):
        if size >= 1024:
            largest[symbol] = max(largest.get(symbol, 0), size)
    symbols = list(largest)
    names = dict(zip(symbols, demangle(symbols)))
    rows = sorted(
        ((size, re.sub(r"::h[0-9a-f]{16}$", "", names[s])) for s, size in largest.items()),
        reverse=True,
    )
    over = [(size, name) for size, name in rows if size > args.budget and args.crates in name]
    for size, name in rows[: args.top]:
        mark = "!" if size > args.budget and args.crates in name else " "
        print(f"{mark} {size:>8} {name[:180]}")
    if over:
        print(f"\n{len(over)} function(s) over the {args.budget}-byte budget", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
