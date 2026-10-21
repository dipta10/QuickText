"""Linux desktop regression check; requires a window manager and xdotool.

Build with `npm run build` and
`cargo build --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol`,
then run `python3 scripts/check-window-visibility.py`. Uses isolated settings/IPC,
never starts recording, and terminates its own app when finished.
"""

import concurrent.futures
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time


def wait_for(check, message):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


binary = Path(__file__).resolve().parents[1] / "src-tauri/target/debug/quicktext"
with tempfile.TemporaryDirectory(prefix="quicktext-window-check-") as directory:
    env = dict(os.environ, GDK_BACKEND="x11", XDG_RUNTIME_DIR=directory,
               XDG_CONFIG_HOME=directory + "/config", XDG_DATA_HOME=directory + "/data")

    def xdo(*args):
        return subprocess.run(["xdotool", *map(str, args)], env=env,
                              capture_output=True, text=True, timeout=5)

    def cli(command):
        output = subprocess.check_output([str(binary), command, "--json"],
                                         env=env, text=True, timeout=20)
        response = json.loads(output)
        assert response["ok"], response
        return response

    with tempfile.TemporaryFile(mode="w+") as log:
        app = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
        try:
            endpoint = Path(directory) / "quicktext.sock"
            wait_for(endpoint.exists, "App did not open its isolated IPC socket")

            def main_window():
                for window in xdo("search", "--pid", app.pid).stdout.splitlines():
                    if xdo("getwindowname", window).stdout.strip() == "QuickText":
                        return window

            window = wait_for(main_window, "Main window did not appear")
            initial = cli("status")

            def focused():
                # WebKit may focus a child X11 window; xdotool resolves its
                # ancestor with WM_CLASS when -f is omitted.
                return xdo("getwindowfocus").stdout.strip() == window

            def visible():
                return window in xdo("search", "--onlyvisible", "--pid", app.pid).stdout.splitlines()

            # Establish that this desktop can focus this window independently
            # before attributing a failed show/focus operation to QuickText.
            xdo("windowactivate", "--sync", window)
            wait_for(focused, "Desktop could not activate the test window")
            assert cli("toggle-window")["visible"] is False
            wait_for(lambda: not visible(), "Window did not hide")
            assert cli("toggle-window")["visible"] is True
            wait_for(visible, "Window did not show")
            wait_for(focused, "Showing a hidden window did not focus it")
            print("PASS: hidden window is shown and focused")

            xdo("windowminimize", window)

            def minimized():
                state = subprocess.check_output(["xprop", "-id", window, "WM_STATE"],
                                                env=env, text=True, timeout=5)
                return "Iconic" in state

            wait_for(minimized, "Desktop did not minimize the test window")
            assert cli("toggle-window")["visible"] is True
            wait_for(visible, "Minimized window did not show")
            wait_for(focused, "Minimized window did not regain focus")
            print("PASS: minimized window is restored and focused")

            def request(_):
                with socket.socket(socket.AF_UNIX) as connection:
                    connection.settimeout(20)
                    connection.connect(str(endpoint))
                    connection.sendall(b'{"v":1,"cmd":"toggle_window"}\n')
                    return json.loads(connection.makefile().readline())

            with concurrent.futures.ThreadPoolExecutor(2) as pool:
                for _ in range(30):
                    replies = list(pool.map(request, range(2)))
                    assert all(reply["ok"] for reply in replies), replies
                    assert sorted(reply["visible"] for reply in replies) == [False, True], replies
            assert cli("status") == initial, "Window requests changed capture state"
            print("PASS: 30 simultaneous request pairs alternate visibility; capture is unchanged")
        finally:
            app.terminate()
            try:
                app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait(timeout=5)
