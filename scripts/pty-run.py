#!/usr/bin/env python3
"""Runs a command on a pseudo-terminal, so `[ -t 0 ]` is true and /dev/tty opens, and types an
answer when the output asks a question. Replaces util-linux `script` (absent on some distros).

    pty-run.py ANSWER PROMPT -- command args...

ANSWER is typed (with a newline) the first time PROMPT appears in the output. The command's output
goes to stdout, and its exit status is returned. Used by scripts/update-test.sh for get.sh's
"in a terminal" cases.
"""

import os
import pty
import select
import sys


def main():
    answer, prompt = sys.argv[1], sys.argv[2].encode()
    command = sys.argv[4:]
    pid, fd = pty.fork()
    if pid == 0:
        os.execvp(command[0], command)
    seen = b""
    answered = False
    while True:
        ready, _, _ = select.select([fd], [], [], 60)
        if not ready:
            os.kill(pid, 9)
            print("pty-run: timed out", file=sys.stderr)
            return 124
        try:
            data = os.read(fd, 4096)
        except OSError:
            break
        if not data:
            break
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()
        seen += data
        if not answered and prompt in seen:
            answered = True
            os.write(fd, answer.encode() + b"\n")
    _, status = os.waitpid(pid, 0)
    return os.waitstatus_to_exitcode(status)


if __name__ == "__main__":
    sys.exit(main())
