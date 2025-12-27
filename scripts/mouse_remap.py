#!/usr/bin/env python3
import evdev
from evdev import ecodes
import subprocess
import os
import sys
import logging
import threading
import time

# Setup logging
log_path = "/tmp/mouse_remap.log"
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler(log_path),
        logging.StreamHandler(sys.stdout)
    ]
)

# Mapping configuration from capture
# Top button (BTN_EXTRA) = 276 -> Copy (Ctrl+C)
# Bottom button (BTN_SIDE) = 275 -> Paste (Ctrl+V)
TOP_BUTTON = 276
BOTTOM_BUTTON = 275

def run_ydotool_key(combo):
    # combo is a list of strings like ["29:1", "46:1", "46:0", "29:0"]
    cmd = ["ydotool", "key"] + combo
    try:
        logging.debug(f"Running ydotool command: {cmd}")
        subprocess.run(cmd, check=True, capture_output=True)
    except Exception as e:
        logging.error(f"Error running ydotool: {e}")

def monitor_device(path):
    try:
        device = evdev.InputDevice(path)
        logging.info(f"Started monitoring {device.name} at {path}")
        for event in device.read_loop():
            if event.type == ecodes.EV_KEY:
                if event.value == 1: # Key Down
                    if event.code == TOP_BUTTON:
                        logging.info(f"Top button pressed on {device.name} -> Copying")
                        # Ctrl(29) + C(46)
                        run_ydotool_key(["29:1", "46:1", "46:0", "29:0"])
                    elif event.code == BOTTOM_BUTTON:
                        logging.info(f"Bottom button pressed on {device.name} -> Pasting")
                        # Ctrl(29) + V(47)
                        run_ydotool_key(["29:1", "47:1", "47:0", "29:0"])
    except Exception as e:
        logging.error(f"Error monitoring {path}: {e}")

def main():
    logging.info("Searching for mouse devices...")
    discovered_paths = []
    
    # Check all available input devices
    for path in evdev.list_devices():
        try:
            device = evdev.InputDevice(path)
            capabilities = device.capabilities()
            if ecodes.EV_KEY in capabilities:
                keys = capabilities[ecodes.EV_KEY]
                # If it has either of the side buttons
                if TOP_BUTTON in keys or BOTTOM_BUTTON in keys:
                    logging.info(f"Found suitable device: {device.name} at {path}")
                    discovered_paths.append(path)
        except Exception as e:
            logging.warn(f"Could not check device {path}: {e}")

    if not discovered_paths:
        logging.error("No devices found match the required button capabilities.")
        sys.exit(1)

    threads = []
    for path in discovered_paths:
        t = threading.Thread(target=monitor_device, args=(path,), daemon=True)
        t.start()
        threads.append(t)

    logging.info(f"Monitoring {len(threads)} device(s).")
    
    # Keep the script running
    try:
        while True:
            time.sleep(10)
    except KeyboardInterrupt:
        logging.info("Shutting down...")

if __name__ == "__main__":
    main()
