"""Renode capture sequence for Barcode Generator. Written but not yet run.

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app barcode --script ../precursor-barcode/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os
import time


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_menu.png")
    ctl.inject_line("")                 # Enter: "New barcode"
    time.sleep(1.0)
    ctl.inject_line("Hello, World!")    # type and Enter: Code 128
    time.sleep(2.0)
    shot("02_code128.png")
    ctl.timed_key("BackSpace", after=1.0)   # back to input
    for _ in range(13):
        ctl.timed_key("BackSpace", after=0.3)
    ctl.inject_line("4006381333931")    # EAN-13
    time.sleep(2.0)
    shot("03_ean13.png")
    ctl.timed_key("BackSpace", after=1.0)
    for _ in range(13):
        ctl.timed_key("BackSpace", after=0.3)
    ctl.inject_line("4006381333932")    # wrong check digit: error stays on input
    time.sleep(1.0)
    shot("04_check_digit_error.png")
    ctl.timed_key("F4", after=1.0)      # back to menu
    shot("05_menu_again.png")
    return failures
