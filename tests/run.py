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
    # test("management_pass", [executable, join(src_dir, "management_pass/main.adept")], compiles)

    for entry in sorted(scandir(success_dir), key=lambda ent: ent.name):
        if not entry.is_dir():
            continue

        folder = join(success_dir, entry.name)
        is_mod = exists(join(folder, "_.adept"))

        cmd = [executable]

        if is_mod:
            cmd.append(folder)
        else:
            cmd.append(join(folder, "main.adept"))

        compiles = lambda _: True
        test(entry.name, cmd, compiles)

    for entry in sorted(scandir(error_dir), key=lambda ent: ent.name):
        if not entry.is_dir():
            continue

        folder = join(error_dir, entry.name)
        is_mod = exists(join(folder, "_.adept"))

        cmd = [executable]

        if is_mod:
            cmd.append(folder)
        else:
            cmd.append(join(folder, "main.adept"))

        compiles = lambda _: True
        test(entry.name, cmd, compiles, expected_exitcode="non-zero")

e2e_framework_run(run_all_tests)
