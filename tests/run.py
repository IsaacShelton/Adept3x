#!/usr/bin/env python

import sys
from os.path import join, dirname, abspath, exists
from os import scandir
from framework import test, e2e_framework_run

e2e_root_dir = dirname(abspath(__file__))
success_dir = join(e2e_root_dir, "success")
error_dir = join(e2e_root_dir, "error")

def run_all_tests():
    executable = sys.argv[1]

    for entry in sorted(scandir(success_dir), key=lambda ent: ent.name):
        if not entry.is_dir():
            continue

        cmd = prepare(executable, success_dir, entry)
        test(entry.name, cmd, compiles)

    for entry in sorted(scandir(error_dir), key=lambda ent: ent.name):
        if not entry.is_dir():
            continue

        cmd = prepare(executable, error_dir, entry)
        test(entry.name, cmd, compiles, expected_exitcode="non-zero")

def prepare(executable, dir, entry):
    folder = join(dir, entry.name)
    is_mod = exists(join(folder, "_.adept"))

    cmd = [executable, "--infrastructure", "infrastructure"]

    if is_mod:
        cmd.append(folder)
    else:
        cmd.append(join(folder, "main.adept"))

    return cmd


def compiles(_):
    return True


e2e_framework_run(run_all_tests)
