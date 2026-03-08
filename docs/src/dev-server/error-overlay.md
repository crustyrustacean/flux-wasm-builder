# Error Overlay

When a build fails during a watch cycle, the error message is sent to all connected browser tabs via WebSocket. The browser displays a full-screen overlay showing the error.

The overlay:

- Appears immediately when a build failure is detected
- Shows the full error message from the build tool
- Is dismissed automatically when the next successful build completes
- Does not require a page refresh to appear or disappear

This means you can keep your browser and editor side by side — build errors surface in the browser without needing to switch to the terminal.

SCSS compilation failures during a watch cycle are also surfaced via the overlay.
