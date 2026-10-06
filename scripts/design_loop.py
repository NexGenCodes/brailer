#!/usr/bin/env python3
"""design_loop.py — prove the core brailer claim: an LLM-authored spec can pass
`brailer verify`, and when it fails, the failure report drives a fix loop.

Two modes:
  A) MEASURE (default): run every `*.json` draft in a directory through the real
     gate and report per-spec, per-frame verdicts and an aggregate pass rate.

       python3 scripts/design_loop.py drafts/            # verify existing drafts
       python3 scripts/design_loop.py drafts/ --json     # machine-readable

  B) GENERATE + MEASURE: if an API key is present, ask an LLM to write a first
     draft for each brief in `--briefs`, write it to `--drafts`, then measure.

       ANTHROPIC_API_KEY=... python3 scripts/design_loop.py --briefs docs/briefs --drafts drafts/

Honesty contract: the pass rate here is the real number between "an LLM writes
a spec" and "brailer says green". First-draft failures are data, not bugs —
the loop is: draft -> verify -> fix -> green. The green specs are the corpus.

The IDs used are self-contained JSON specs (no external images or assets).
"""

import argparse
import json
import os
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

BRAILER = os.environ.get("BRAILER", os.path.join("target", "release", "brailer"))


def verify(spec: Path, binary: str) -> list[dict]:
    """Run EVERY frame the spec declares; return one parsed report per frame."""
    out = subprocess.run(
        [binary, "verify", str(spec), "--json"],
        capture_output=True, text=True,
    )
    if out.returncode not in (0, 1, 2):
        return [{"frame": "?", "ok": False, "fatal": out.stderr.strip()}]
    try:
        lines = [l for l in out.stdout.splitlines() if l.strip()[:1] in "{["]
        if not lines:
            # exit 2: rejected before layout — errors are in stderr or stdout text
            text = (out.stdout + "\n" + out.stderr).strip()
            return [{"frame": "?", "ok": False, "fatal": text[:300]}]
        reports = []
        for l in lines:
            data = json.loads(l)
            if isinstance(data, list):
                reports.extend(data)
            elif isinstance(data, dict):
                reports.append(data)
        return reports
    except json.JSONDecodeError:
        return [{"frame": "?", "ok": False,
                 "fatal": (out.stdout + out.stderr).strip()[:300]}]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--drafts", default="scripts/drafts")
    ap.add_argument("--briefs", default="docs/briefs")
    ap.add_argument("--model", default="claude-sonnet-4-5")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("drafts_pos", nargs="?", default=None,
                    help="positional shortcut for --drafts")
    args = ap.parse_args()
    drafts = Path(args.drafts_pos or args.drafts)
    briefs = Path(args.briefs)

    binary = BRAILER
    if not Path(binary).exists():
        print(f"error: {binary} not found (cargo build --release first)",
              file=sys.stderr)
        return 2

    # ---- Mode B: generate first drafts from briefs via an LLM ----
    key = os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("OPENAI_API_KEY")
    brief_files = sorted(briefs.glob("*.md"))
    if key and brief_files and not drafts.exists():
        _generate(briefs, drafts, args.model, key)

    if not drafts.is_dir():
        print(f"no drafts dir: {drafts}", file=sys.stderr)
        return 2

    specs = sorted(drafts.glob("*.json"))
    if not specs:
        print(f"no spec drafts in {drafts}", file=sys.stderr)
        return 2

    rows, frames, passes, fails = [], 0, 0, 0
    problems = {}
    for spec in specs:
        reports = verify(spec, binary)
        frames += len(reports)
        any_ok = all(r.get("ok") for r in reports)
        passes += int(any_ok)
        fails += int(not any_ok)
        seen = {f for f in (r.get("frame") for r in reports)}
        detail = ", ".join(
            f"{r.get('frame')}:{'PASS' if r.get('ok') else 'FAIL'}"
            for r in reports)
        for r in reports:
            if not r.get("ok"):
                for k in ("errors", "out_of_bounds", "collisions", "contrast", "fatal"):
                    n = r.get(k)
                    if n:
                        problems.setdefault(k, 0)
                        problems[k] += len(n) if isinstance(n, list) else 1
        rows.append({
            "file": spec.name,
            "frames": sorted(seen),
            "ok": any_ok,
            "detail": detail,
        })

    if args.json:
        print(json.dumps({
            "drafts": [r["file"] for r in rows],
            "frames": frames,
            "pass": passes,
            "fail": fails,
            "first_pass_rate": round(passes / len(specs), 3) if specs else None,
            "frame_pass_rate": round(passes / frames, 3) if frames else None,
            "problems": problems,
            "per_spec": rows,
        }, indent=2))
        return 0

    print(f"{'draft':<24} {'frames':<18} verdict")
    print("-" * 60)
    for r in rows:
        print(f"{r['file']:<24} {', '.join(r['frames']):<18} "
              f"{'PASS' if r['ok'] else 'FAIL'}")
    print("-" * 60)
    total = passes + fails
    rate = passes / total if total else 0.0
    print(f"\nspecs: {total}   frames: {frames}   "
          f"first-pass: {passes}/{total} ({rate:.0%})")
    if problems:
        print("problem inventory across failing drafts:")
        for k, n in sorted(problems.items(), key=lambda kv: -kv[1]):
            print(f"  {k}: {n}")

    # Never render anything that did not all pass — same contract as `render`.
    return 0 if passes == total else 1


def _generate(briefs: Path, drafts: Path, model: str, key: str) -> None:
    """Have an LLM write one first-draft spec per brief (best-effort, offline
    without a key). Never records a draft that fails the gate — green specs are
    written back only if they pass, so the corpus stays convergent."""
    try:
        import urllib.request
    except ImportError:
        return
    drafts.mkdir(parents=True, exist_ok=True)
    skill = Path("SKILL.md").read_text() if Path("SKILL.md").exists() else ""
    if "ANTHROPIC" in (os.environ.get("ANTHROPIC_API_KEY") or ""):
        url, auth, model_field = "https://api.anthropic.com/v1/messages", "anthropic-api-key", "model"
        prompt_key = "prompt"
    else:
        url, auth, model_field = "https://api.openai.com/v1/chat/completions", "Authorization", "model"
        prompt_key = "messages"
    for brief in sorted(briefs.glob("*.md")):
        system = (skill + "\n\nReturn ONLY a single JSON document: "
                  "one theme, desktop+mobile frames, no comments, no markdown.")
        body = {
            model_field: model,
            "system": system,
            "prompt": f"Design brief:\n\n{brief.read_text()}\n"
                      f"\nEmit the JSON spec now.",
        }
        req = urllib.request.Request(
            url, data=json.dumps(body).encode(),
            headers={"Content-Type": "application/json", auth: key},
        )
        try:
            resp = json.loads(urllib.request.urlopen(req, timeout=60).read())
            text = (resp.get("content", [{}])[0].get("text")
                    or resp["choices"][0]["message"]["content"])
            spec = _strip_fence(text)
            out = drafts / (brief.stem + ".json")
            out.write_text(spec)
            ok = all(r.get("ok") for r in verify(out, BRAILER))
            if not ok:
                out.unlink(missing_ok=True)
        except Exception as exc:  # offline / no quota — never fail the run
            print(f"  skip {brief.name}: {exc}", file=sys.stderr)


def _strip_fence(text: str) -> str:
    text = text.strip()
    if text.startswith("```"):
        lines = text.splitlines()
        lines = lines[1:(-1 if lines and lines[-1].startswith("```") else None)]
        text = "\n".join(lines).strip()
    return text


if __name__ == "__main__":
    sys.exit(main())