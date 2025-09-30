#!/usr/bin/env python3
"""
Combine all file contents under a source tree into a single text file,
excluding a specific subdirectory.

Usage:
    python3 combine_src_to_text.py
    # or customize paths by editing DEFAULT_* constants or passing CLI args:
    python3 combine_src_to_text.py "/path/to/src" "/path/to/src/bin" "/path/to/output.txt"
"""

from __future__ import annotations
import sys
import os
from pathlib import Path
from typing import Iterable

# --- Default locations (edit these if you want to run without CLI args) ---
DEFAULT_SRC_DIR = Path("/Users/JulioContreras/Desktop/School/Research/Baseball SuPro /SuPro Rewritten/src")
DEFAULT_EXCLUDE_DIR = DEFAULT_SRC_DIR / "bin"
DEFAULT_OUTPUT_FILE = DEFAULT_SRC_DIR / "_all_source.txt"
DEFAULT_EXTRA_PATHS = [
    Path("/Users/JulioContreras/Desktop/School/Research/Baseball SuPro /SuPro Rewritten/Cargo.toml"),
    Path("/Users/JulioContreras/Desktop/School/Research/Baseball SuPro /SuPro Rewritten/build.sh"),
]
# -------------------------------------------------------------------------


def iter_files(root: Path, exclude_dir: Path) -> Iterable[Path]:
    """
    Yield all files under `root`, skipping anything inside `exclude_dir`.
    """
    root = root.resolve()
    exclude_dir = exclude_dir.resolve()
    for dirpath, dirnames, filenames in os.walk(root):
        current = Path(dirpath).resolve()

        # If we're at/inside the excluded folder, prune it from traversal.
        # (Modify dirnames in-place so os.walk won't descend.)
        # Compare using .resolve() to be robust to symlinks.
        dirnames[:] = [
            d for d in dirnames
            if (Path(dirpath) / d).resolve() != exclude_dir
            and not d.startswith(".git")   # nice to skip VCS internals if present
        ]

        # Skip everything if this subtree is the excluded path
        if current == exclude_dir or exclude_dir in current.parents:
            continue

        for fn in filenames:
            p = current / fn
            # Skip typical junk files
            if fn == ".DS_Store":
                continue
            yield p


def read_file_as_text(path: Path) -> str:
    """
    Read a file as UTF-8 text, falling back to replacement characters on decode errors.
    If the file is truly binary, you'll still get a readable best-effort dump.
    """
    try:
        # Try straightforward UTF-8 first
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        # Fallback: decode with replacement to avoid crashing
        with path.open("rb") as f:
            data = f.read()
        return data.decode("utf-8", errors="replace")
    except Exception as e:
        return f"<<ERROR READING FILE: {e}>>"


def write_combined_output(
    src_dir: Path,
    exclude_dir: Path,
    out_file: Path,
    extra_paths: Iterable[Path] | None = None,
) -> None:
    """
    Create (or overwrite) `out_file` containing the concatenated contents of all files
    under `src_dir` except those inside `exclude_dir`. Each file is prefixed by a header
    showing its relative path. If `extra_paths` are provided, append them at the end
    with absolute-path headers.
    """
    src_dir = src_dir.resolve()
    exclude_dir = exclude_dir.resolve()
    out_file = out_file.resolve()

    out_file.parent.mkdir(parents=True, exist_ok=True)

    files = sorted(iter_files(src_dir, exclude_dir))
    count = 0

    with out_file.open("w", encoding="utf-8") as out:
        out.write(f"# Combined dump of {src_dir}\n")
        out.write(f"# Excluding: {exclude_dir}\n\n")

        for p in files:
            rel = p.relative_to(src_dir)
            out.write("\n")
            out.write("=" * 80 + "\n")
            out.write(f"FILE: {rel}\n")
            out.write("=" * 80 + "\n\n")
            out.write(read_file_as_text(p))
            out.write("\n")
            count += 1

        # --- append extras at the very end ---
        if extra_paths:
            out.write("\n")
            out.write("#" * 80 + "\n")
            out.write("# EXTRA FILES (appended after source tree)\n")
            out.write("#" * 80 + "\n\n")

            for xp in extra_paths:
                xp = Path(xp).resolve()
                out.write("\n")
                out.write("=" * 80 + "\n")
                out.write(f"EXTRA FILE: {xp}\n")
                out.write("=" * 80 + "\n\n")
                out.write(read_file_as_text(xp))
                out.write("\n")

    print(f"Wrote {count} in-tree files + {len(list(extra_paths or []))} extras into: {out_file}")


def main() -> None:
    """
    Entry point. You can:
      - Run with no args to use the DEFAULT_* constants at the top, or
      - Provide 1–N args:
            arg1 = src_dir
            arg2 = exclude_dir (defaults to <src_dir>/bin if omitted)
            arg3 = output_file (defaults to <src_dir>/_all_source.txt if omitted)
            arg4..N = extra file paths to append at end
    """
    argv = sys.argv[1:]

    if len(argv) == 0:
        src_dir = DEFAULT_SRC_DIR
        exclude_dir = DEFAULT_EXCLUDE_DIR
        out_file = DEFAULT_OUTPUT_FILE
        extra_paths = DEFAULT_EXTRA_PATHS
    else:
        src_dir = Path(argv[0])
        exclude_dir = Path(argv[1]) if len(argv) >= 2 else (src_dir / "bin")
        out_file = Path(argv[2]) if len(argv) >= 3 else (src_dir / "_all_source.txt")
        extra_paths = [Path(p) for p in argv[3:]] if len(argv) >= 4 else DEFAULT_EXTRA_PATHS

    write_combined_output(src_dir, exclude_dir, out_file, extra_paths)


if __name__ == "__main__":
    main()
