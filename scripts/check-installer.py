#!/usr/bin/env python3
"""Every program the Windows installer runs is named by its full path:

    scripts/check-installer.py [packaging/windows/installer.nsi]

Windows looks for a program named bare first in the folder the program
starting it was loaded from — for an installer, Downloads as a rule — so
a `taskkill.exe` or `cmd.exe` left there ran in place of Windows' own
(SEC-001). Each `Exec`, `ExecWait`, `ExecShell` and `nsExec::Exec*`
command, and each program a `cmd /c` in one runs, must start from
`$SYSDIR`, `$WINDIR`, `$INSTDIR` or `$PLUGINSDIR`."""
import re, sys

NSI = sys.argv[1] if len(sys.argv) > 1 else "packaging/windows/installer.nsi"

EXEC = re.compile(r"^\s*(nsExec::Exec\w*|ExecWait|ExecShellWait|ExecShell|Exec)\s+(.*)$", re.I)
STRING = re.compile(r"""'([^']*)'|"((?:[^"$]|\$\\"|\$(?!\\"))*)"|`([^`]*)`""")
QUALIFIED = re.compile(r"^\$(SYSDIR|WINDIR|INSTDIR|PLUGINSDIR)\\", re.I)


def first_word(cmd):
    """The program a command line starts, and what follows it."""
    cmd = cmd.lstrip()
    if cmd.startswith('"'):
        end = cmd.find('"', 1)
        return (cmd[1:end], cmd[end + 1:]) if end > 0 else (cmd[1:], "")
    word = re.match(r"\S*", cmd).group(0)
    return word, cmd[len(word):]


def programs(cmd):
    """The programs `cmd` runs: its first word and, after `cmd /c`, the
    first word of each command in what cmd is given to run."""
    first, rest = first_word(cmd)
    yield first
    name = re.split(r"[\\/]", first)[-1].lower()
    rest = rest.strip()
    if name in ("cmd", "cmd.exe") and rest[:2].lower() == "/c":
        inner = rest[2:].strip()
        # cmd drops the outer pair of quotes after /c.
        if len(inner) > 1 and inner[0] == '"' and inner[-1] == '"':
            inner = inner[1:-1]
        for piece in re.split(r"\|\|?|&&?", inner):
            if piece.strip():
                yield first_word(piece)[0]


def command_of(instruction, args):
    """The command line an Exec-family instruction runs, or None."""
    strings = [next(g for g in m.groups() if g is not None) for m in STRING.finditer(args)]
    if instruction.lower().startswith("execshell"):
        # ExecShell verb command [parameters]: the program is the second.
        return strings[1] if len(strings) > 1 else None
    return strings[0] if strings else None


def unqualified(lines):
    """(line number, program) for each program run by a bare name."""
    found = []
    for n, line in enumerate(lines, 1):
        # A comment is a whole line; a `;` inside a command is not one.
        if line.lstrip().startswith((";", "#")):
            continue
        m = EXEC.match(line)
        if not m:
            continue
        cmd = command_of(m.group(1), m.group(2))
        if cmd is None:
            found.append((n, "<no command string>"))
            continue
        for prog in programs(cmd):
            if not QUALIFIED.match(prog):
                found.append((n, prog))
    return found


# The lines SEC-001 was about, and their fix: a check that let the first
# through, or flagged the second, would be checking nothing.
BAD = [
    "  nsExec::Exec 'taskkill /im anima-engine.exe'",
    """    nsExec::Exec 'cmd /c tasklist /fi "imagename eq anima-engine.exe" | find /i "anima-engine.exe"'""",
    """    nsExec::Exec '"$SYSDIR\\cmd.exe" /c ""$SYSDIR\\tasklist.exe" | find /i "x""'""",
    '  ExecWait "notepad.exe"',
    '  ExecShell "open" "cmd.exe"',
]
GOOD = [
    """  nsExec::Exec '"$SYSDIR\\taskkill.exe" /im anima-engine.exe'""",
    """    nsExec::Exec '"$SYSDIR\\cmd.exe" /c ""$SYSDIR\\tasklist.exe" /fi "imagename eq anima-engine.exe" | "$SYSDIR\\find.exe" /i "anima-engine.exe""'""",
    "  ExecWait '\"$INSTDIR\\uninstall.exe\" /S'",
    "  ; nsExec::Exec 'taskkill' in a comment",
    "  # nsExec::Exec 'taskkill' in a comment",
]
for line in BAD:
    assert unqualified([line]), f"self-check: not flagged: {line}"
for line in GOOD:
    assert not unqualified([line]), f"self-check: flagged: {line} -> {unqualified([line])}"

lines = open(NSI, encoding="utf-8").read().splitlines()
runs = sum(1 for line in lines if EXEC.match(line) and not line.lstrip().startswith((";", "#")))
bad = unqualified(lines)
for n, prog in bad:
    print(f"{NSI}:{n}: runs {prog!r} by a bare name; name it by its full path ($SYSDIR\\…)")
print(f"== {runs} commands, {len(bad)} by a bare name")
sys.exit(1 if bad or runs == 0 else 0)
