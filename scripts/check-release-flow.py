#!/usr/bin/env python3
"""release.yml's hand-offs, run as written, before a tag is pushed:

    scripts/check-release-flow.py .github/workflows/release.yml v1.5.0

Each job's `run:` steps that move files (rename, stage, checksums) run
verbatim in a temporary folder on stub artifacts named as the build
scripts name them; the artifacts pass between the jobs as
upload/download-artifact v4 pass them (`name`, then `pattern` +
`merge-multiple` into one folder); the attest subject paths and the
files `gh release create` names are then expanded against what arrived.
The builds themselves are not run. Needs PyYAML."""
import fnmatch, glob, os, re, shutil, subprocess, sys, tempfile
import yaml

WF = sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/release.yml"
TAG = sys.argv[2] if len(sys.argv) > 2 else "v1.5.0-rc1"
VER = TAG[1:]
DEBVER = VER.replace("-", "~")
wf = yaml.safe_load(open(WF))
jobs = wf["jobs"]
results = []

def check(label, ok, detail=""):
    results.append(bool(ok))
    print(f"{'ok ' if ok else 'BAD'} {label}{(' -- ' + str(detail)) if detail else ''}")

def step(job, name):
    for s in jobs[job]["steps"]:
        if s.get("name", "").startswith(name):
            return s
    raise SystemExit(f"no step {name!r} in {job}")

def run(script, cwd):
    env = {**os.environ, "GITHUB_REF_NAME": TAG}
    r = subprocess.run(["bash", "-eo", "pipefail", "-c", script], cwd=cwd, env=env,
                       capture_output=True, text=True)
    if r.returncode:
        print(r.stdout, r.stderr)
    return r.returncode == 0

root = tempfile.mkdtemp(prefix="handoff-")
artifacts = os.path.join(root, "artifact-server")

def upload(job, ws):
    s = next(s for s in jobs[job]["steps"] if s.get("uses", "").startswith("actions/upload-artifact"))
    name, path = s["with"]["name"], s["with"]["path"]
    dest = os.path.join(artifacts, name)
    shutil.copytree(os.path.join(ws, path), dest)
    print(f"   {job}: uploaded {name}: {sorted(os.listdir(dest))}")
    return name

# ── build (Linux) ──────────────────────────────────────────────────────
ws = os.path.join(root, "build"); os.makedirs(os.path.join(ws, "build"))
for f in (f"anima-engine_{DEBVER}-1_amd64.deb", f"animaEngine-{VER}-x86_64.AppImage"):
    open(os.path.join(ws, "build", f), "w").write(f)
check("build: rename to GitHub-safe names", run(step("build", "Rename artifacts")["run"], ws))
check("build: stage", run(step("build", "Stage release artifacts")["run"], ws))
up_linux = upload("build", ws)
check("build has no checksums step any more (publish does them)",
      not any(s.get("name", "").startswith("Checksums") for s in jobs["build"]["steps"]))

# ── build-windows ──────────────────────────────────────────────────────
check("build-windows waits for build", "build" in str(jobs["build-windows"].get("needs")))
check("build-windows runs on 24.04", jobs["build-windows"]["runs-on"] == "ubuntu-24.04")
ws = os.path.join(root, "win"); os.makedirs(os.path.join(ws, "build"))
for f in (f"animaEngine-{VER}-windows-x86_64.zip", f"animaEngine-{VER}-windows-x86_64-setup.exe"):
    open(os.path.join(ws, "build", f), "w").write(f)
check("build-windows: stage", run(step("build-windows", "Stage release artifacts")["run"], ws))
up_win = upload("build-windows", ws)

# ── publish ────────────────────────────────────────────────────────────
pub = jobs["publish"]
check("publish waits for both", set(pub["needs"]) == {"build", "build-windows"}, pub["needs"])
dl = next(s for s in pub["steps"] if s.get("uses", "").startswith("actions/download-artifact"))
pattern, merge, path = dl["with"].get("pattern"), dl["with"].get("merge-multiple"), dl["with"]["path"]
ws = os.path.join(root, "publish"); os.makedirs(ws)
got = [n for n in sorted(os.listdir(artifacts)) if fnmatch.fnmatch(n, pattern)]
check("the pattern takes both artifacts", got == sorted([up_linux, up_win]), got)
dest = os.path.join(ws, path); os.makedirs(dest)
for n in got:
    src = os.path.join(artifacts, n)
    target = dest if merge else os.path.join(dest, n)
    os.makedirs(target, exist_ok=True)
    for f in os.listdir(src):
        shutil.copy(os.path.join(src, f), os.path.join(target, f))
check("merge-multiple: all four files side by side", merge is True and len(os.listdir(dest)) == 4,
      sorted(os.listdir(dest)))
check("publish: checksums", run(step("publish", "Checksums")["run"], ws))
sums = open(os.path.join(dest, f"SHA256SUMS-{TAG}.txt")).read().split("\n")
check("the sums cover the four", len([l for l in sums if l.strip()]) == 4)
check("sha256sum -c passes", run(f"cd {path} && sha256sum -c SHA256SUMS-{TAG}.txt", ws))
att = next(s for s in pub["steps"] if s.get("uses", "").startswith("actions/attest-build-provenance"))
subjects = [p.strip() for p in att["with"]["subject-path"].split("\n") if p.strip()]
matched = {p: glob.glob(os.path.join(ws, p)) for p in subjects}
check("every attest subject path matches one file", all(len(m) == 1 for m in matched.values()),
      {p: [os.path.basename(x) for x in m] for p, m in matched.items()})
create = step("publish", "Create draft release")["run"]
files = [w for w in re.findall(r'"?(dist/[^\s"\\]+)"?', create)]
expanded = [glob.glob(os.path.join(ws, f.replace("$GITHUB_REF_NAME", TAG))) for f in files]
check("every file gh release create names exists", all(len(e) == 1 for e in expanded),
      [os.path.basename(e[0]) if e else f for e, f in zip(expanded, files)])
shutil.rmtree(root)
print(f"== {sum(results)}/{len(results)} as expected")
sys.exit(0 if all(results) else 1)
